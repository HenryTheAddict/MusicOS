import {useEffect, useState} from 'react';
import {errorText} from './bridge';
import {desktopUpdateApi, type InstallProgress, type UpdateStatus} from './desktopUpdateApi';

export default function SoftwareUpdates() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [update, setUpdate] = useState<string | null>(null);
  const [result, setResult] = useState<'current' | null>(null);
  const [phase, setPhase] = useState<'idle' | 'checking' | InstallProgress['phase'] | 'restarting'>('idle');
  const [download, setDownload] = useState<Extract<InstallProgress, {phase: 'downloading'}> | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let active = true;
    desktopUpdateApi.status().then(value => { if (active) setStatus(value); }).catch(() => {});
    return () => { active = false; };
  }, []);

  if (!status) return null;

  async function check() {
    setPhase('checking');
    setError('');
    setUpdate(null);
    setResult(null);
    try {
      const next = await desktopUpdateApi.check();
      setUpdate(next);
      if (!next) setResult('current');
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setPhase('idle');
    }
  }

  async function install() {
    if (!update) return;
    setPhase('downloading');
    setDownload(null);
    setError('');
    try {
      await desktopUpdateApi.install(update, progress => {
        setPhase(progress.phase);
        if (progress.phase === 'downloading') setDownload(progress);
      });
      setPhase('restarting');
    } catch (reason) {
      setError(errorText(reason));
      setPhase('idle');
    }
  }

  const message = phase === 'checking' ? 'Checking GitHub Releases…'
    : phase === 'downloading' ? 'Downloading the update…'
    : phase === 'installing' ? 'Verifying and installing the update…'
    : phase === 'restarting' ? 'Restarting musicOS…'
    : update ? `Version ${update} is available. Installing will restart musicOS.`
    : result === 'current' ? 'You’re up to date.'
    : '';

  return <article className="settings-tile software-updates" aria-labelledby="software-updates-title">
    <div className="setting-inline">
      <div className="settings-summary">
        <h2 id="software-updates-title">Software updates</h2>
        <p>Installed version {status.version}</p>
      </div>
      {status.available && <button className="soft-action" type="button" disabled={phase !== 'idle'}
        onClick={update ? install : check}>{update ? 'Install & restart' : 'Check for updates'}</button>}
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
