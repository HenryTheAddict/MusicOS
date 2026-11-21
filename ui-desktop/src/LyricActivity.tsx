import { useEffect, useRef, useState } from 'react';
import { Icon } from './components';
import { clockPosition, currentPlaybackClock, usePlaybackClock } from './playbackFeed';

type LyricActivityProps = {
  loading?: boolean;
  progressBar?: boolean;
  active?: boolean;
  playing?: boolean;
  start?: number;
  end?: number;
};

export default function LyricActivity({
  loading = false,
  progressBar = false,
  active = true,
  playing = false,
  start = 0,
  end = 0,
}: LyricActivityProps) {
  const clock = usePlaybackClock(active);
  const bar = useRef<HTMLProgressElement>(null);
  const [visible, setVisible] = useState(() => !document.hidden);

  useEffect(() => {
    const change = () => setVisible(!document.hidden);
    document.addEventListener('visibilitychange', change);
    return () => document.removeEventListener('visibilitychange', change);
  }, []);

  useEffect(() => {
    const element = bar.current;
    if (!element || !progressBar || !playing || !active || !visible || end <= start) return;
    // Keep the bar moving between clock messages, with the same clock as the lyrics.
    const timer = window.setInterval(() => {
      const elapsed = clockPosition(currentPlaybackClock()) * 1000 - start;
      element.value = Math.max(0, Math.min(1, elapsed / (end - start)));
    }, 100);
    return () => clearInterval(timer);
  }, [progressBar, playing, active, visible, start, end]);

  const positionMs = clockPosition(clock) * 1000;
  const progress = end > start ? Math.max(0, Math.min(1, (positionMs - start) / (end - start))) : 0;
  if (progressBar) {
    return <progress ref={bar} className="lyric-break-progress" aria-label="Instrumental break progress" max={1} value={progress} />;
  }
  return (
    <span
      className="lyric-activity"
      role={loading ? 'status' : undefined}
      aria-label={loading ? 'Finding lyrics' : 'Instrumental break'}
      data-loading={loading || undefined}
      data-moving={(active && visible && (loading || playing)) || undefined}
    >
      {!loading && <Icon name="Song" />}
      <span className="lyric-activity-dots" aria-hidden="true">
        {[0, 1, 2].map(dot => <i key={dot} data-filled={(!loading && progress * 3 > dot) || undefined} />)}
      </span>
    </span>
  );
}
