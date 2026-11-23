import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { AssetIcon } from './AssetIcon';
import { call, errorText, subscribe } from './bridge';
import type { Task } from './types';

export interface OrganizationPreview {
  token: string;
  moves: { id: number; from: string; to: string }[];
  skipped: number;
  undo_available: number;
  issues?: { path: string; message: string }[];
}

type Action = 'preview' | 'apply' | 'undo' | 'recover';
type Phase = Action | 'refresh';
const phaseLabel: Record<Phase, string> = {
  preview: 'Checking files for your preview',
  apply: 'Moving files',
  undo: 'Restoring files',
  recover: 'Checking interrupted file work',
  refresh: 'Refreshing your library',
};

function files(count: number) {
  return `${count} ${count === 1 ? 'file' : 'files'}`;
}

function splitPath(path: string) {
  const windows = /^(?:[a-z]:|\\\\)/i.test(path);
  const separator = Math.max(path.lastIndexOf('/'), windows ? path.lastIndexOf('\\') : -1);
  return {
    folder: path.slice(0, separator + (separator === 0 || (windows && separator === 2) ? 1 : 0)),
    name: path.slice(separator + 1),
  };
}

export default function FileOrganizer({ onClose, onRefresh }: {
  onClose: () => void;
  onRefresh: () => Promise<void>;
}) {
  const titleId = useId();
  const descriptionId = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const active = useRef<{ action: Action; stopRequested: boolean; cancelled: boolean } | null>(null);
  const [phase, setPhase] = useState<Phase | null>('preview');
  const [progress, setProgress] = useState<{ processed: number; total: number } | null>(null);
  const [stopping, setStopping] = useState(false);
  const [preview, setPreview] = useState<OrganizationPreview | null>(null);
  const [error, setError] = useState('');
  const [canRecover, setCanRecover] = useState(false);
  const [message, setMessage] = useState('');
  const busy = phase !== null;

  const start = useCallback((action: Action) => {
    const operation = { action, stopRequested: false, cancelled: false };
    active.current = operation;
    setPhase(action);
    setProgress(null);
    setStopping(false);
    setError('');
    setCanRecover(false);
    return operation;
  }, []);

  const loadPreview = useCallback(async () => {
    const operation = start('preview');
    setPreview(null);
    try {
      const result = await call('organize_files', { action: 'preview' });
      if (!('moves' in result)) throw new Error('Unable to load organization preview');
      if (operation.cancelled || operation.stopRequested) return;
      setPreview(result);
    } catch (e) {
      setError(errorText(e));
    } finally {
      active.current = null;
      setPhase(null);
    }
  }, [start]);

  useLayoutEffect(() => {
    const node = dialog.current!;
    const opener = document.activeElement;
    node.showModal();
    heading.current?.focus();
    return () => {
      node.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);

  useLayoutEffect(() => {
    if (!dialog.current?.contains(document.activeElement)) heading.current?.focus();
  }, [phase]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void subscribe<Task>('task', task => {
      const operation = active.current;
      if (disposed || task.kind !== 'organize' || !operation) return;
      if (task.running && task.total > 0) setProgress(task);
      if (!task.running && task.cancelled) operation.cancelled = true;
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch(() => {
      // The preview still works without progress events.
    }).then(() => {
      if (!disposed) void loadPreview();
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [loadPreview]);

  async function runChange(action: Exclude<Action, 'preview'>) {
    if (active.current || busy) return;
    const operation = start(action);
    setMessage('');
    try {
      const result = await call('organize_files', { action, token: preview?.token });
      if (!('count' in result)) throw new Error('Unexpected organization response');
      const wasStopped = action === 'apply' && (operation.cancelled || operation.stopRequested) &&
        result.count < (preview?.moves.length ?? 0);
      setMessage(action === 'recover' ? 'Interrupted file work checked.'
        : action === 'undo' ? `${files(result.count)} restored.`
        : wasStopped ? `Stopped. ${files(result.count)} organized.`
        : `${files(result.count)} organized.`);
    } catch (e) {
      active.current = null;
      setPreview(null);
      setError(errorText(e));
      setCanRecover(true);
      setPhase(null);
      return;
    }
    active.current = null;
    setPreview(null);
    setPhase('refresh');
    setProgress(null);
    setStopping(false);
    try {
      await onRefresh();
    } catch (e) {
      setError(errorText(e));
      setPhase(null);
      return;
    }
    await loadPreview();
  }

  async function stop() {
    const operation = active.current;
    if (!operation || operation.stopRequested || !['preview', 'apply'].includes(operation.action)) return;
    operation.stopRequested = true;
    setStopping(true);
    setError('');
    try {
      await call('cancel_task');
    } catch (e) {
      if (active.current !== operation) return;
      operation.stopRequested = false;
      setStopping(false);
      setError(errorText(e));
    }
  }

  const count = preview?.moves.length ?? 0;
  const issues = preview?.issues ?? [];
  const undoCount = preview?.undo_available ?? 0;
  const canStop = phase === 'preview' || phase === 'apply';
  const progressLabel = phase ? phaseLabel[phase] : '';

  return (
    <dialog ref={dialog} className="file-organizer" aria-labelledby={titleId} aria-describedby={descriptionId}
      onCancel={event => { event.preventDefault(); if (!busy) onClose(); }}>
      <header className="organization-header">
        <span className="organization-glyph"><AssetIcon name="Storage" /></span>
        <div>
          <h2 id={titleId} ref={heading} tabIndex={-1}>Smartly organize my files</h2>
          <p id={descriptionId}>Move originals into Artist / Album folders inside each music folder.</p>
        </div>
        <button type="button" className="organization-close" aria-label="Close file organizer" disabled={busy} onClick={onClose}>
          <AssetIcon name="controlClose" />
        </button>
      </header>
      {!busy && preview && (
        <header className="organization-review">
          <h3>{count ? `${files(count)} to move` : 'No files to move'}</h3>
          {preview.skipped > 0 && <p>{files(preview.skipped)} left in place · managed or unavailable</p>}
        </header>
      )}
      <div className="organization-body" role={count && !busy ? 'region' : undefined}
        aria-label={count && !busy ? 'File move preview' : undefined} tabIndex={count && !busy ? 0 : undefined}>
        {busy && (
          <div className="organization-progress">
            <div className="organization-progress-heading">
              <p role="status">{stopping ? 'Stopping after the current file…' : `${progressLabel}…`}</p>
              {progress && <span aria-hidden="true">{progress.processed} / {progress.total}</span>}
            </div>
            <div className="organization-rail" role="progressbar" aria-label={progressLabel}
              aria-valuemin={0} aria-valuemax={progress?.total} aria-valuenow={progress?.processed}>
              <div className="organization-fill" style={progress ? { width: `${100 * progress.processed / progress.total}%` } : undefined} />
            </div>
          </div>
        )}
        {error && <p className="organization-notice" role="alert">{error}</p>}
        {!busy && message && <p className="organization-notice" role="status">{message}</p>}
        {!busy && !preview && !error && <p role="status">Preview stopped. No files were moved by the preview.</p>}
        {!busy && issues.length > 0 && (
          <details className="organization-issues">
            <summary>{files(issues.length)} could not be moved</summary>
            <ul>
              {issues.map(issue => <li key={issue.path}><strong>{issue.path}</strong><span>{issue.message}</span></li>)}
            </ul>
          </details>
        )}
        {!busy && count > 0 && (
          <ul className="organization-moves" role="list" aria-label="Proposed moves">
            {preview!.moves.map(move => {
              const from = splitPath(move.from);
              const to = splitPath(move.to);
              return (
                <li key={move.id}>
                  <strong>{from.name}</strong>
                  <dl>
                    <dt>From</dt><dd>{from.folder}</dd>
                    <dt>To</dt><dd>{from.name === to.name ? to.folder : move.to}</dd>
                  </dl>
                </li>
              );
            })}
          </ul>
        )}
      </div>
      <footer className="organization-footer">
        {!busy && preview && count > 0 && (
          <p>Pause playback before moving. Tags, companion files and empty folders stay unchanged; existing files aren’t replaced.</p>
        )}
        <div className="organization-actions">
          {busy ? (
            canStop && <button type="button" disabled={stopping} onClick={() => void stop()}>
              {phase === 'preview' ? 'Stop preview' : 'Stop after this file'}
            </button>
          ) : (
            <>
              {canRecover && <button type="button" onClick={() => void runChange('recover')}>Retry interrupted file work</button>}
              {undoCount > 0 && <button type="button" onClick={() => void runChange('undo')}>Undo organization ({undoCount})</button>}
              {!preview ? (
                <button type="button" className="organization-primary" onClick={() => void loadPreview()}>Preview again</button>
              ) : count > 0 ? (
                <button type="button" className="organization-primary" onClick={() => void runChange('apply')}>Move {files(count)}</button>
              ) : (
                <button type="button" className="organization-primary" onClick={onClose}>Done</button>
              )}
            </>
          )}
        </div>
      </footer>
    </dialog>
  );
}
