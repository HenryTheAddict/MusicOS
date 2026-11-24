import { useEffect, useId, useRef, useState, type ComponentProps, type CSSProperties } from 'react';
import { listen } from '@tauri-apps/api/event';
import BeatPulseGrid from './BeatPulseGrid';
import MovingIcons from './MovingIcons';
import ExpandedPlayer from './ExpandedPlayer';
import { call, native, errorText } from './bridge';
import { beatOffsets } from './heartVisibility';
import { defaultVisualizer, isWebVisualizer, movingIconsPreset, type AlbumPalette, type VisualizerSettings, type VisualizerStatus } from './visualizer';
import { useVisualizerAlbumPalette } from './visualizerAlbumPalette';
import VisualizerPicker from './VisualizerPicker';
import VisualizerSettingsPanel from './VisualizerSettings';
import type { Preferences } from './types';
import './visualizer.css';

const stills = import.meta.glob<string>('../../assets/visualizers/thumbnails/*.png', {
  query: '?url', import: 'default', eager: true,
});
const reducedMotion = '(prefers-reduced-motion: reduce)';
let windowOperation = Promise.resolve<unknown>(undefined);
// Keep a reloaded webview newer than its previous in-flight native requests.
let nextRequestId = Date.now() * 1000;

function serializeWindow(work: () => Promise<unknown>) {
  const next = windowOperation.then(work);
  windowOperation = next.catch(() => {});
}

function VisualizerStill({ preset, palette }: { preset: string; palette?: AlbumPalette }) {
  const filter = useId();
  return (
    <>
      {palette && (
        <svg width="0" height="0" aria-hidden="true">
          <filter id={filter} colorInterpolationFilters="sRGB">
            <feColorMatrix type="saturate" values="0" />
            <feComponentTransfer>
              {(['feFuncR', 'feFuncG', 'feFuncB'] as const).map((Channel, index) => (
                <Channel key={Channel} type="table" tableValues={[0, ...palette.map(color => color[index]), 1].join(' ')} />
              ))}
            </feComponentTransfer>
          </filter>
        </svg>
      )}
      <img
        className="visualizer-still"
        style={palette ? { filter: `url(#${filter})` } : undefined}
        src={stills[`../../assets/visualizers/thumbnails/${preset}.png`]}
        alt=""
        aria-hidden="true"
      />
    </>
  );
}

function VisualizerError({ message, onRetry }: { message: string; onRetry: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const details = useId();
  return (
    <div className="visualizer-unavailable" role="alert" aria-label={message} title={message} data-details={expanded || undefined}>
      <strong>Visualizer unavailable</strong>
      <button className="soft-action visualizer-error-toggle" aria-expanded={expanded} aria-controls={details}
        onClick={() => setExpanded(value => !value)}>Details</button>
      <p id={details} role={expanded ? 'region' : undefined} aria-label={expanded ? 'Visualizer error details' : undefined}
        tabIndex={expanded ? 0 : undefined}>{message}</p>
      <button className="soft-action" onClick={onRetry}>Retry</button>
    </div>
  );
}

type VisualizerPlayerProps = ComponentProps<typeof ExpandedPlayer> & {
  preferences: Preferences;
  onPreferences: (preferences: Preferences) => void;
};

export default function VisualizerPlayer({ preferences, onPreferences, ...props }: VisualizerPlayerProps) {
  const [reduced, setReduced] = useState(() => matchMedia(reducedMotion).matches);
  const [ready, setReady] = useState(!native);
  const [status, setStatus] = useState<VisualizerStatus | null>(null);
  const [error, setError] = useState('');
  const [attempt, setAttempt] = useState(0);
  const prefs = preferences.visualizer ?? defaultVisualizer;
  const beatPulse = prefs.preset === 'beat-pulse';
  const webPreset = isWebVisualizer(prefs.preset);
  const latest = useRef({ preferences, onPreferences });
  latest.current = { preferences, onPreferences };

  const { player, bpm } = props;
  const trackId = player.track?.id;
  const offset = trackId === undefined ? undefined : beatOffsets.get(trackId);
  const artwork = useVisualizerAlbumPalette(true, trackId, props.cover);
  const albumPalette = prefs.album_colors ? artwork.palette : undefined;
  const albumPalettePending = Boolean(prefs.album_colors && artwork.pending);
  const webPalette = useRef(albumPalette);
  if (!albumPalettePending) webPalette.current = albumPalette;

  const chromeColor = useRef<string | undefined>(undefined);
  if (!artwork.pending) {
    const chroma = (rgb: number[]) => Math.max(...rgb) - Math.min(...rgb);
    const dominant = artwork.palette?.reduce((best, swatch) => chroma(swatch) > chroma(best) ? swatch : best);
    chromeColor.current = dominant && chroma(dominant) > .08
      ? '#' + dominant.map(channel => Math.round(channel * 255).toString(16).padStart(2, '0')).join('')
      : undefined;
  }
  const theme = { ...props.theme, '--visualizer-source': chromeColor.current ?? '#858996' } as CSSProperties;

  const beat = { trackId, bpm, offset, confidence: offset === undefined ? 0 : 0.5 };
  const queueEntry = player.queue_entry_ids?.[player.queue_index ?? -1];
  const identity = JSON.stringify([
    prefs, albumPalette, albumPalettePending, beat, reduced, player.is_playing, player.listening?.speed, queueEntry,
  ]);
  const revision = useRef({ identity: '', id: 0 });
  if (revision.current.identity !== identity) revision.current = { identity, id: ++nextRequestId };
  const requestId = revision.current.id;
  const settings = useRef<VisualizerSettings>(null!);
  settings.current = { requestId, preferences: prefs, albumPalette, albumPalettePending, beat, reduced };
  const restoreNativeWindow = useRef<(() => Promise<void>) | undefined>(undefined);

  useEffect(() => {
    let alive = true;
    let unsubscribe: (() => void) | undefined;
    let openedNativeSurface = false;
    const previousBrowserFull = document.fullscreenElement;

    async function restore() {
      if (openedNativeSurface) await call('visualizer', { request: { action: 'close' } }).catch(() => {});
      if (!native && document.fullscreenElement !== previousBrowserFull) {
        if (previousBrowserFull) await previousBrowserFull.requestFullscreen().catch(() => {});
        else if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
      }
      delete document.documentElement.dataset.nativeVisualizer;
    }

    async function open() {
      if (!alive) return;
      try {
        if (native && !restoreNativeWindow.current) {
          const { getCurrentWindow } = await import('@tauri-apps/api/window');
          const window = getCurrentWindow();
          const wasFullscreen = await window.isFullscreen();
          if (!alive) return;
          if (!wasFullscreen) await window.setFullscreen(true);
          restoreNativeWindow.current = () => window.setFullscreen(wasFullscreen);
          if (!alive) return;
        }

        if (webPreset) {
          setStatus(null);
        } else {
          if (!native) throw Error('Native Metal visualizers are available in the macOS desktop app.');
          const stop = await listen<VisualizerStatus>('musicos-visualizer-status', ({ payload }) => {
            if (!alive || payload.requestId !== settings.current.requestId) return;
            setStatus(payload);
            if (payload.error) setError(payload.error);
            const { preferences, onPreferences } = latest.current;
            const visualizer = preferences.visualizer ?? defaultVisualizer;
            if (payload.preset && payload.preset !== visualizer.preset) {
              onPreferences({ ...preferences, visualizer: { ...visualizer, preset: payload.preset } });
            }
          });
          if (!alive) {
            stop();
            return;
          }
          unsubscribe = stop;
          openedNativeSurface = true;
          document.documentElement.dataset.nativeVisualizer = 'true';
          const status = await call('visualizer', { request: { action: 'open', settings: settings.current } });
          if (alive && status.requestId === settings.current.requestId) setStatus(status);
        }
        if (alive) setReady(true);
      } catch (failure) {
        if (alive) {
          setError(errorText(failure));
          setReady(true);
        }
      }
    }

    setError('');
    setReady(!native);
    serializeWindow(open);
    return () => {
      alive = false;
      unsubscribe?.();
      serializeWindow(restore);
    };
  }, [attempt, webPreset]);

  useEffect(() => () => {
    if (!native) return;
    serializeWindow(async () => {
      const restore = restoreNativeWindow.current;
      restoreNativeWindow.current = undefined;
      await restore?.();
    });
  }, []);

  useEffect(() => {
    if (webPreset || !native || !ready || document.documentElement.dataset.nativeVisualizer !== 'true') return;
    let alive = true;
    void call('visualizer', { request: { action: 'refresh', settings: settings.current } }).catch(failure => {
      if (alive && requestId === settings.current.requestId) setError(errorText(failure));
    });
    return () => { alive = false; };
  }, [webPreset, ready, requestId]);

  useEffect(() => {
    const media = matchMedia(reducedMotion);
    const update = () => setReduced(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  const save = (visualizer: typeof prefs) => onPreferences({ ...preferences, visualizer });
  const preset = webPreset ? prefs.preset : status?.preset || prefs.preset;
  return (
    <ExpandedPlayer
      {...props}
      theme={theme}
      onView={next => {
        props.onView(next);
        save({ ...prefs, lyrics: next === 'lyrics' });
      }}
      visualizerMode
      visualizerCover={prefs.show_cover !== false}
      coverPulse={prefs.beat_sync}
      fullscreenManaged
      visualizerControls={(
        <>
          <VisualizerPicker value={preset} onChange={preset => save({ ...prefs, preset })} />
          <VisualizerSettingsPanel
            value={prefs}
            fallback={Boolean(status?.beatFallback)}
            onChange={visualizer => {
              save(visualizer);
              if (visualizer.lyrics !== prefs.lyrics) props.onView(visualizer.lyrics ? 'lyrics' : 'playing');
            }}
          />
        </>
      )}
      visualizerStatus={(
        <>
          {beatPulse && (
            <BeatPulseGrid
              beatSync={prefs.beat_sync}
              bpm={bpm}
              offset={offset}
              trackId={trackId}
              playing={player.is_playing}
              active={props.open}
              reduced={reduced}
              palette={webPalette.current}
            />
          )}
          {prefs.preset === movingIconsPreset && (
            <MovingIcons trackId={trackId} playing={player.is_playing} active={props.open} reduced={reduced} palette={webPalette.current} />
          )}
          {!webPreset && reduced && <VisualizerStill preset={preset} palette={albumPalette} />}
          {error ? (
            <VisualizerError message={error} onRetry={() => {
              void call('visualizer', { request: { action: 'close' } }).finally(() => setAttempt(value => value + 1));
            }} />
          ) : (!ready || status?.phase === 'starting') && (
            <div className="visualizer-starting" role="status">Preparing visualizer…</div>
          )}
        </>
      )}
    />
  );
}
