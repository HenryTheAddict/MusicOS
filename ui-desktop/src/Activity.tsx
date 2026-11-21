import { useEffect, useRef, useState } from 'react';
import { call, errorText } from './bridge';
import { Icon } from './components';

export type ActivityJob = {
  id: string;
  kind: string;
  title: string;
  state: string;
  completed: number;
  total: number;
  message: string;
  finished_at?: number | null;
};

const statusLabels: Record<string, string> = {
  running: 'In progress', complete: 'Complete', cancelled: 'Cancelled', interrupted: 'Interrupted',
};

export default function Activity({ onClose, onResults }: { onClose: () => void; onResults: (kind: string) => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const request = useRef(0);
  const refresh = useRef<() => Promise<void>>(async () => {});
  const [jobs, setJobs] = useState<ActivityJob[] | null>(null);
  const [error, setError] = useState('');
  const [cancelling, setCancelling] = useState<string | null>(null);

  useEffect(() => {
    const element = dialog.current;
    const opener = document.querySelector<HTMLElement>('.section-actions-trigger');
    if (element && !element.open) element.showModal();
    let active = true;
    let timer = 0;

    refresh.current = async () => {
      clearTimeout(timer);
      const id = ++request.current;
      try {
        const next = await call('activity_jobs');
        if (!active || id !== request.current) return;
        setJobs(next);
        setError('');
      } catch (failure) {
        if (active && id === request.current) setError(errorText(failure));
      } finally {
        if (active && id === request.current) timer = window.setTimeout(refresh.current, 1500);
      }
    };
    void refresh.current();
    return () => {
      active = false;
      clearTimeout(timer);
      element?.close();
      if (opener?.isConnected) opener.focus({ preventScroll: true });
    };
  }, []);

  async function cancel(job: ActivityJob) {
    setCancelling(job.id);
    try {
      if (job.kind === 'apple') await call('apple_enrichment', { request: { action: 'stop_cache' } });
      else if (job.kind === 'wifi') await call('wifi_sync', { action: 'stop' });
      else await call('cancel_task');
      await refresh.current();
    } catch (failure) {
      setError(errorText(failure));
    } finally {
      setCancelling(null);
    }
  }

  return (
    <dialog ref={dialog} className="song-selector activity-panel" aria-labelledby="activity-title" onCancel={event => {
      event.preventDefault();
      onClose();
    }}>
      <header>
        <h2 id="activity-title">Activity</h2>
        <button autoFocus aria-label="Close Activity" onClick={onClose}><Icon name="controlClose" /></button>
      </header>
      {error && <div className="activity-error" role="alert"><p>{error}</p><button onClick={() => void refresh.current()}>Refresh</button></div>}
      <div className="activity-list" aria-busy={jobs === null && !error}>
        {jobs?.map(job => (
          <article key={job.id}>
            <div><strong>{job.title}</strong><small>{statusLabels[job.state] ?? 'Needs attention'}</small></div>
            {job.message && <p>{job.message}</p>}
            {job.state === 'running' && <>
              <progress aria-label={job.title} max={job.total || 1} value={job.total ? job.completed : undefined} />
              <button disabled={cancelling !== null} onClick={() => void cancel(job)}>
                {cancelling === job.id ? 'Cancelling…' : 'Cancel'}
              </button>
            </>}
            {job.state !== 'running' && ['enrich', 'apple', 'wifi'].includes(job.kind) && (
              <button onClick={() => onResults(job.kind)}>{job.state === 'complete' ? 'Open results' : 'Open tools'}</button>
            )}
          </article>
        ))}
        {!error && jobs === null && <p role="status">Loading activity…</p>}
        {!error && jobs?.length === 0 && <p>No recent activity.</p>}
      </div>
    </dialog>
  );
}
