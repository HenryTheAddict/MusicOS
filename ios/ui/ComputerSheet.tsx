import {useEffect, useRef, useState} from 'react';
import {AssetIcon as Icon} from '../../ui-desktop/src/AssetIcon';
import {errorText, request} from './bridge';

type SyncPhase = 'idle' | 'starting' | 'connecting' | 'approval' | 'preparing' | 'downloading' | 'updating' | 'uploading' | 'cancelling' | 'cancelled' | 'complete' | 'error';
type SyncStatus = {
 paired: boolean;
 nearby_macs: {id: string; name: string}[];
 offline_songs: number;
 storage_bytes: number;
 pending_changes: number;
 conflicts: {id: string; operation: {name?: string}}[];
 sync: {state: SyncPhase; message?: string; completed?: number; total?: number; unit?: 'songs' | 'files'};
};
const busyPhases = new Set<SyncPhase>(['starting', 'connecting', 'preparing', 'downloading', 'updating', 'uploading', 'cancelling']);
const phaseTitle: Record<SyncPhase, string> = {
 idle: 'Ready to sync', starting: 'Starting sync', connecting: 'Connecting to your Mac',
 approval: 'Waiting for approval', preparing: 'Preparing songs on your Mac', downloading: 'Copying music',
 updating: 'Updating your offline library', uploading: 'Sending your changes', cancelling: 'Stopping sync',
 cancelled: 'Sync stopped', complete: 'Ready offline', error: 'Sync needs attention',
};

export default function ComputerSheet({paired, onClose, onLibraryChange,onRemote}: {
 paired: boolean; onClose: () => void; onLibraryChange: () => Promise<void>; onRemote:()=>void;
}) {
 const [status, setStatus] = useState<SyncStatus>();
 const [link, setLink] = useState('');
 const [pending, setPending] = useState<string>();
 const [error, setError] = useState('');
 const [disconnect, setDisconnect] = useState(false);
 const serial = useRef(0);

 async function refreshStatus() {
  const revision = ++serial.current;
  const result: SyncStatus = await request({action: 'sync_status'});
  if (revision === serial.current) setStatus(result);
 }

 useEffect(() => {
  let active = true;
  let timer: number;
  void request({action: 'discover'}).catch(() => {});
  async function poll() {
   if (!document.hidden) await refreshStatus().catch(() => {});
   if (active) timer = window.setTimeout(poll, 850);
  }
  void poll();
  return () => {
   active = false;
   ++serial.current;
   clearTimeout(timer);
   void request({action: 'stop_discovery'}).catch(() => {});
  };
 }, []);

 async function run(action: string, args: Record<string, unknown> = {}) {
  if (pending) return;
  setPending(action);
  setError('');
  try {
   const result = await request({action, ...args});
   if (action === 'scan' && result?.link) setLink(result.link);
   if (action === 'pair') setLink('');
   if (action === 'disconnect') setDisconnect(false);
   await refreshStatus();
   if (['pair', 'disconnect', 'resolve'].includes(action)) await onLibraryChange();
  } catch (cause) {
   setError(errorText(cause));
  } finally {
   setPending(undefined);
  }
 }

 const isPaired = status?.paired ?? paired;
 const phase = status?.sync.state ?? 'idle';
 const syncing = busyPhases.has(phase) || pending === 'sync';
 const total = status?.sync.total ?? 0;
 const completed = status?.sync.completed ?? 0;
 const unit = status?.sync.unit ?? 'songs';
 const message = status?.sync.message;
 const retry = ['approval', 'error', 'cancelled'].includes(phase);

 return <section className="phone-sheet phone-computer" role="dialog" aria-modal="true" aria-label="Computer">
  <div className="section-title"><h1>Computer</h1><button className="icon-button" aria-label="Close Computer" onClick={onClose}><Icon name="controlClose"/></button></div>
  {isPaired ? <>
   <div className="computer-sync-card">
    <div className="computer-sync-heading"><Icon name="System"/><h2>Wi-Fi sync</h2></div>
    <div role="status" className="computer-sync-status" data-phase={phase}>
     <strong>{pending === 'sync' && !busyPhases.has(phase) ? 'Starting sync' : phaseTitle[phase]}</strong>
     {message && message !== phaseTitle[phase] && <p>{message}</p>}
    </div>
    {syncing && <div className="computer-sync-progress">
     <progress aria-label={phaseTitle[phase]} max={total || 1} value={total ? Math.min(completed, total) : undefined}/>
     {total > 0 && <small>{completed} of {total} {unit}</small>}
    </div>}
    <div className="actions">
     {syncing ? <button className="filled" disabled={!!pending || phase === 'cancelling'} onClick={() => void run('cancel')}>Stop sync</button>
      : <button className="filled primary" disabled={!!pending} onClick={() => void run('sync')}><Icon name="sync"/>{retry ? 'Retry sync' : 'Sync now'}</button>}
    </div>
   </div>
   {status && <div className="computer-offline"><h2>On this iPhone</h2><p>{status.offline_songs} {status.offline_songs === 1 ? 'song' : 'songs'} offline · {new Intl.NumberFormat(undefined, {style:'unit', unit:status.storage_bytes >= 1e9 ? 'gigabyte' : 'megabyte', maximumFractionDigits:1}).format(status.storage_bytes / (status.storage_bytes >= 1e9 ? 1e9 : 1e6))}</p>
    {status.pending_changes > 0 && <small>{status.pending_changes} {status.pending_changes === 1 ? 'change waiting' : 'changes waiting'} to sync to your Mac</small>}
   </div>}
   {status?.conflicts.map(conflict => <div className="conflict" key={conflict.id}><h2>Changes on both devices</h2><p>{conflict.operation.name ?? 'Favorite changed'}</p><div className="actions">
    <button className="filled" disabled={!!pending || syncing} onClick={() => void run('resolve', {id:conflict.id, phone:false})}>Use Mac</button>
    <button className="filled" disabled={!!pending || syncing} onClick={() => void run('resolve', {id:conflict.id, phone:true})}>Use iPhone</button>
   </div></div>)}
   <section className="computer-offline"><h2>Mac Remote</h2><button className="filled" onClick={onRemote}><Icon name="System"/>Control playback on Mac</button></section>
   <details className="computer-manage"><summary>Manage connection</summary>
    {disconnect ? <><p>Disconnect this Mac? Your downloaded songs stay on this iPhone.</p><div className="actions">
     <button className="filled" disabled={!!pending} onClick={() => setDisconnect(false)}>Keep connection</button>
     <button className="filled" disabled={!!pending || syncing} onClick={() => void run('disconnect')}>Disconnect</button>
    </div></> : <button className="filled compact" disabled={syncing || !!pending} onClick={() => setDisconnect(true)}>Disconnect Mac</button>}
   </details>
  </> : <>
   <div className="device-art" aria-hidden="true"><Icon name="System"/><Icon name="sync"/><Icon name="iPhoneDynamic"/></div>
   <h2>Connect your Mac</h2>
   <p>On your Mac, open Devices and create a pairing code. Keep both devices on the same Wi-Fi network.</p>
   {!!status?.nearby_macs?.length && <p className="nearby-macs">Nearby: {status.nearby_macs.map(mac => mac.name).join(', ')}</p>}
   <button className="filled primary" disabled={!!pending} onClick={() => void run('scan')}><Icon name="iPhoneDynamic"/>Scan pairing code</button>
   <details className="computer-manage" open={link ? true : undefined}><summary>Use a pairing link</summary>
    <label htmlFor="pair-link">Pairing link</label>
    <input id="pair-link" value={link} onChange={event => setLink(event.target.value)} placeholder="musicos://pair?…" autoCapitalize="none" autoCorrect="off"/>
   </details>
   {link && <button className="filled primary" disabled={!!pending} onClick={() => void run('pair', {link:link.trim()})}>{pending === 'pair' ? 'Pairing…' : 'Pair computer'}</button>}
  </>}
  {error && <p className="computer-error" role="alert">{error}</p>}
 </section>;
}
