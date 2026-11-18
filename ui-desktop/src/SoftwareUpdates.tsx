import {useEffect, useState} from 'react';
import {errorText} from './bridge';
import {desktopUpdateApi, type DownloadProgress, type UpdateStatus} from './desktopUpdateApi';

export default function SoftwareUpdates({audioPlaying}: {audioPlaying: boolean}) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [update, setUpdate] = useState<string | null>(null);
  const [ready, setReady] = useState<string | null>(null);
  const [current, setCurrent] = useState(false);
  const [phase, setPhase] = useState<'idle' | 'checking' | DownloadProgress['phase'] | 'restarting'>('idle');
  const [download, setDownload] = useState<Extract<DownloadProgress, {phase: 'downloading'}> | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    desktopUpdateApi.status().then(value => {
      if (!active) return;
      setStatus(value);
      setReady(value?.ready ?? null);
      setError(value?.error ?? '');
    }).catch(() => {});
    return () => { active = false; };
  }, []);

  if (!status) return null;

  async function check() {
    setPhase('checking');
    setError('');
    setCurrent(false);
    try {
      const next = await desktopUpdateApi.check();
      setUpdate(next);
      setCurrent(!next);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setPhase('idle');
    }
  }

  async function downloadUpdate() {
    if (!update) return;
    setPhase('downloading');
    setDownload(null);
    setError('');
    try {
      await desktopUpdateApi.download(update, progress => {
        setPhase(progress.phase);
        if (progress.phase === 'downloading') setDownload(progress);
      });
      setReady(update);
      setUpdate(null);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setPhase('idle');
    }
  }

  async function restart() {
    if (!ready || audioPlaying) return;
    setPhase('restarting');
    setError('');
    try {
      await desktopUpdateApi.restart(ready);
    } catch (reason) {
      setError(errorText(reason));
      const next = await desktopUpdateApi.status().catch(() => null);
      if (next) setReady(next.ready);
      setPhase('idle');
    }
  }

  const message = phase === 'checking' ? 'Checking GitHub Releases…'
    : phase === 'downloading' ? 'Downloading the update…'
    : phase === 'verifying' ? 'Verifying the update…'
    : phase === 'restarting' ? 'Installing and restarting musicOS…'
    : ready ? `Version ${ready} is ready. ${audioPlaying ? 'Pause playback to restart, or update next time you open musicOS.' : 'Restart now or update next time you open musicOS.'}`
    : update ? `Version ${update} is available. Downloading won’t interrupt playback.`
    : current ? 'You’re up to date.'
    : '';

  return <article className="settings-tile software-updates" aria-labelledby="software-updates-title">
    <div className="setting-inline">
      <div className="settings-summary">
        <h2 id="software-updates-title">Software updates</h2>
        <p>Installed version {status.version}</p>
      </div>
      {status.available && <button className="soft-action" type="button"
        disabled={phase !== 'idle' || (!!ready && audioPlaying)}
        onClick={ready ? restart : update ? downloadUpdate : check}>
        {ready ? 'Restart to update' : update ? 'Download update' : 'Check for updates'}
      </button>}
    </div>
    {status.available ? <>
      <p role="status" className="software-update-status">{message}</p>
      {phase === 'downloading' && <progress aria-label="Update download"
        max={download?.total || undefined}
        value={download?.total ? Math.min(download.received, download.total) : undefined}/>}
      {error && <p role="alert" className="inline-error">{error}</p>}
    </> : <p className="software-update-status">In-app updates are available in supported release builds.</p>}
  </article>;
}
