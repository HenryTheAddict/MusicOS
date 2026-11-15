import { useEffect, useState, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { call, errorText, native } from "./bridge";
import { Icon } from "./components";
import "./devicesWifi.css";
import {useViewActive} from "./ViewScope";
import ThemedChoice from "./ThemedChoice";
import type { DeviceStatus, SyncPlan, Library, SavedCollection } from "./types";
function EmbeddedDevices({
  onSync,
  busy,
}: {
  onSync: (root: string) => void;
  busy: boolean;
}) {
  const [panel, setPanel] = useState<"transfer" | "preview">("transfer");
  const [status, setStatus] = useState<DeviceStatus | null>(null);
  const [target, setTarget] = useState("http://127.0.0.1:18080");
  const [plan, setPlan] = useState<SyncPlan | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  async function operate(action: string, address?: string) {
    setWorking(true);
    setError("");
    try {
      const result = (await call("device", {
        action,
        target: address,
      })) as DeviceStatus;
      if (action === "status") {
        setStatus(result);
        setTarget(result.target);
      } else {
        const next = (await call("device", {
          action: "status",
        })) as DeviceStatus;
        setStatus(next);
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      setWorking(false);
    }
  }
  useEffect(() => {
    void operate("status");
  }, []);
  useEffect(() => {
    let gone = false;
    let inFlight = false;
    const timer = setInterval(() => {
      if (working || inFlight) return;
      inFlight = true;
      void call("device", { action: "status" })
        .then((result) => {
          if (!gone) setStatus(result as DeviceStatus);
        })
        .catch(() => {})
        .finally(() => {
          inFlight = false;
        });
    }, 3000);
    return () => {
      gone = true;
      clearInterval(timer);
    };
  }, [working]);
  async function planSync() {
    setError("");
    try {
      if (!native) throw Error("Open the desktop app to choose a sync folder.");
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Choose music to sync",
      });
      if (typeof folder !== "string") return;
      setWorking(true);
      setPlan(
        (await call("device", { action: "plan", root: folder })) as SyncPlan,
      );
    } catch (e) {
      setError(errorText(e));
    } finally {
      setWorking(false);
    }
  }
  return (
    <section className="devices">
      <header className="device-toolbar">
        <span>musicOS player</span>
        <span className={`connection ${status?.connected ? "connected" : ""}`}>
          {status?.connected ? "Connected" : "Not connected"}
        </span>
      </header>
      <div className="device-layout">
        <aside className="device-list">
          <header className="device-identity">
            <Icon name="device" />
            <div><h2>musicOS player</h2><p>{status?.connected ? "Ready for your music" : "Waiting for a connection"}</p></div>
          </header>
          <section className="device-settings-panel">
            <h3>Connection</h3>
            <label className="field-label" htmlFor="device-address">Device address</label>
            <input id="device-address" value={target} onChange={(e) => setTarget(e.target.value)} spellCheck={false} />
            <button className="primary" disabled={working} onClick={() => void operate("status", target)}>
              {working ? "Connecting…" : "Connect"}
            </button>
            {status?.error && <small>{status.error}</small>}
          </section>
          <section className="device-settings-panel">
            <h3>Device preview</h3>
            <p>Try musicOS without a connected player.</p>
            <button disabled={working} onClick={() => void operate(status?.preview.running ? "preview_stop" : "preview_start")}>
              <Icon name="autostart" />
              {working ? "Working…" : status?.preview.running ? "Stop preview" : "Launch preview"}
            </button>
            {status?.preview.last_error && <small role="alert">{status.preview.last_error}</small>}
          </section>
          <div className="device-connection-summary">
            <Icon name={status?.connected ? "device" : "NetSearch"} />
            <strong>{status?.connected ? "Connected" : "Offline"}</strong>
          </div>
        </aside>
        <div className="device-panels">
          <div className="device-mode" aria-label="Device tools">
            <button aria-pressed={panel === "transfer"} onClick={() => setPanel("transfer")}><Icon name="Storage" />Music transfer</button>
            <button aria-pressed={panel === "preview"} onClick={() => setPanel("preview")}><Icon name="device" />Device preview</button>
          </div>
          {panel === "transfer" ? <section className="device-transfer">
            <header><h2>Sync your music</h2><button disabled={working || busy} onClick={() => void planSync()}>Review sync</button></header>
            {plan ? <div className="sync-plan">
              <strong>{plan.files} files · {(plan.bytes / 1024 / 1024).toFixed(1)} MB</strong>
              <span className="sync-folder">{plan.root}</span>
              <p>Review this folder, then transfer its music to your player.</p>
              <button disabled={!status?.connected || busy} onClick={() => onSync(plan.root)}>Sync to player</button>
              {!status?.connected && <small>Connect your player to start the transfer.</small>}
            </div> : <div className="device-empty"><Icon name="sync" /><p>Choose a music folder to review before transferring it to your player.</p><small>Your local library stays in place.</small></div>}
          </section> : <section className="device-transfer">
            <header><h2>Device preview</h2></header>
            <div className="device-empty"><Icon name="device" /><p>The embedded musicOS interface runs in its own window.</p><button disabled={working} onClick={() => void operate(status?.preview.running ? "preview_stop" : "preview_start")}>{status?.preview.running ? "Stop preview" : "Launch preview"}</button></div>
          </section>}
          <section className="device-quality"><h2>Transfer format</h2><strong>{plan ? plan.profile.replaceAll("_", " ") : "Choose a folder to review"}</strong><p>{plan ? "Using your saved device quality profile." : "The sync review shows your saved quality profile and transfer size."}</p></section>
        </div>
      </div>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </section>
  );
}

export type CopyFormat='original'|'vorbis256'|'vorbis320';
export interface WifiStatus {
 running:boolean;name?:string;track_count:number;bytes_total:number;skipped:number;selected_ids:number[];format?:CopyFormat;vorbis_available?:boolean;
 preparation?:{preparing:boolean;completed:number;total:number;current:string;error:string|null;format:CopyFormat};
 peers:{id:string;name:string;last_sync_at:number|null;track_count:number;bytes_transferred?:number;listening_allowed?:boolean;remote_allowed?:boolean}[];
 pending:{id:string;name:string;created_at:number;sync_changes?:boolean}[];
 pairing:{link:string;qr:string;expires_at:number}|null;error:string|null;
}
const size=(bytes:number)=>bytes>=1073741824?`${(bytes/1073741824).toFixed(1)} GB`:`${(bytes/1048576).toFixed(1)} MB`;
export default function Devices({library,collections,onSync,busy}:{library:Library;collections:SavedCollection[];onSync:(root:string)=>void;busy:boolean}){
 const [status,setStatus]=useState<WifiStatus|null>(null),[selection,setSelection]=useState(()=>sessionStorage.getItem('musicos-wifi-selection')??'all'),[format,setFormat]=useState<CopyFormat>(()=>{const value=sessionStorage.getItem('musicos-wifi-format');return value==='vorbis256'||value==='vorbis320'?value:'original';}),[working,setWorking]=useState(false),[error,setError]=useState(''),[copied,setCopied]=useState(false),[advanced,setAdvanced]=useState(false),[forget,setForget]=useState<string|null>(null);
 useEffect(()=>{sessionStorage.setItem('musicos-wifi-selection',selection);sessionStorage.setItem('musicos-wifi-format',format);},[selection,format]);
 const active=useViewActive(),operation=useRef(0),pending=useRef(false);
 const selected=new Set(selection==='all'?library.tracks.filter(t=>t.available).map(t=>t.id):selection==='favorites'?library.tracks.filter(t=>t.favorited&&t.available).map(t=>t.id):collections.find(c=>c.id===selection)?.track_ids??[]);
 const ids=[...selected],preparation=status?.preparation,preparing=!!preparation?.preparing;
 const estimated=format==='original'?null:library.tracks.filter(t=>selected.has(t.id)).reduce((n,t)=>n+t.duration*(format==='vorbis256'?256000:320000)/8,0);
 async function operate(action:string,peerId?:string){const request=++operation.current;pending.current=true;setError('');setWorking(true);try{const result=await call('wifi_sync',{action,peerId,trackIds:action==='start'?ids:undefined,copyFormat:action==='start'?format:undefined});if(request===operation.current){setStatus(result);if(action==='pair')setCopied(false);}}catch(e){if(request===operation.current)setError(errorText(e));}finally{if(request===operation.current){pending.current=false;setWorking(false);}}}
 useEffect(()=>{if(!active)return;let gone=false,inFlight=false;async function refresh(){if(inFlight||pending.current)return;inFlight=true;const request=operation.current;try{const next=await call('wifi_sync',{action:'status'});if(!gone&&request===operation.current)setStatus(next);}catch(e){if(!gone&&request===operation.current)setError(errorText(e));}finally{inFlight=false;}}void refresh();const timer=setInterval(()=>{if(!document.hidden)void refresh();},1000);return()=>{gone=true;clearInterval(timer);};},[active]);
 return <section className="wifi-devices">
  <header><div><h1>Devices</h1><p>Copy music to paired devices.</p></div><span className="wifi-state" data-on={status?.running}>{status?.running?(preparing?'Sharing · preparing copies':'Sharing on this network'):'Wi-Fi sharing is off'}</span></header>
  <div className="wifi-workspace">
  <section className="wifi-share"><header><Icon name="Library"/><div><h2>Your offline copy</h2><p>Choose what travels with you.</p></div></header>
   <label>Music<ThemedChoice label="Music to copy" value={selection} disabled={working||preparing} onChange={setSelection} options={[{value:"all",label:"All available music",icon:"Library"},{value:"favorites",label:"Favorites",icon:"hearted"},...collections.map(c=>({value:c.id,label:c.name}))]}/></label>
   <div className="wifi-format"><span>Copy format</span><div role="group" aria-label="Copy format">{([['original','Original'],['vorbis256','Vorbis 256'],['vorbis320','Vorbis 320']] as const).map(([value,label])=><button key={value} aria-pressed={format===value} disabled={working||preparing||(value!=='original'&&status?.vorbis_available===false)} onClick={()=>setFormat(value)}>{label}</button>)}</div><small>{format==='original'?'Keep the original audio quality.':'Ogg Vorbis · '+(format==='vorbis256'?'256':'320')+' kbps target · stereo. First copy needs conversion.'} Originals stay on this Mac.</small>{status?.vorbis_available===false&&<small>Install FFmpeg with Vorbis support to enable smaller copies.</small>}</div>
   <div className="wifi-selection-summary"><strong>{ids.length} {ids.length===1?'song':'songs'}</strong>{estimated!==null&&<span>About {size(estimated)} on your phone</span>}</div>
   {preparing&&<div className="wifi-preparation" role="status"><div><strong>Preparing {preparation!.completed} of {preparation!.total}</strong><span>{preparation!.current||'Opening your library…'}</span></div><progress aria-label="Preparing music" value={preparation!.completed} max={Math.max(1,preparation!.total)}/><small>You can pair your phone now. Unchanged copies are reused next time.</small></div>}
   {!preparing&&status?.running&&status.track_count>0&&<p className="wifi-ready" role="status">{status.format==='vorbis256'?'Vorbis 256':status.format==='vorbis320'?'Vorbis 320':'Original'} · {status.track_count} songs · {size(status.bytes_total)} ready to copy.{status.skipped>0?` ${status.skipped} unavailable files skipped.`:''}</p>}
   <div className="wifi-actions"><button data-playful className="primary" disabled={working||preparing||!ids.length} onClick={()=>void operate('start')}><Icon name="sync"/>{working?'Opening sharing…':preparing?'Preparing music…':status?.running?'Update shared music':'Start Wi-Fi sharing'}</button>{status?.running&&<button disabled={working} onClick={()=>void operate('stop')}>{preparing?'Cancel & stop':'Stop sharing'}</button>}</div>
  </section>
  <section className="wifi-phone"><header><Icon name="device"/><div><h2>Bring your phone</h2><p>Open Settings → Computer in musicOS on your phone.</p></div></header><ol><li>Join the same Wi-Fi as this Mac.</li><li>Scan a pairing link, then approve your phone here.</li><li>Tap Sync now on your phone. Listen offline afterward.</li></ol><button data-playful disabled={working||!status?.running} onClick={()=>void operate('pair')}><Icon name="Plus"/>{status?.pairing?'New pairing link':'Create pairing link'}</button>
   {status?.pairing&&<div className="wifi-pair"><img alt="Pairing QR code" src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(status.pairing.qr)}`}/><div><strong>Scan with your phone’s camera</strong><p>Expires in two minutes. Compare the request before approving.</p><button onClick={()=>void navigator.clipboard.writeText(status.pairing!.link).then(()=>setCopied(true)).catch(()=>setError('Could not copy the pairing link. Try scanning the QR code.'))}>{copied?'Copied':'Copy pairing link'}</button></div></div>}
   {!status?.running&&<p className="wifi-note">Start sharing first. Sharing stays off when musicOS launches.</p>}
   {status?.pending.map(phone=><article className="wifi-request" key={phone.id}><div><strong>{phone.name}</strong><small>Request {phone.id.slice(0,6)} · compare with the phone{phone.sync_changes?" · Favorites and manual playlists sync both ways":""}</small></div><button disabled={working} className="primary" onClick={()=>void operate('approve',phone.id)}>Approve</button><button disabled={working} onClick={()=>void operate('reject',phone.id)}>Decline</button></article>)}
  </section></div>
  <section className="wifi-paired"><h2>Your phones</h2><p className="wifi-note">Allow listening activity to use your Mac’s Last.fm and Discord connections on a paired phone.</p>{status?.peers.length?status.peers.map(phone=><article key={phone.id}><Icon name="device"/><div><strong>{phone.name}</strong><p>{phone.last_sync_at?`${phone.track_count} songs copied · ${new Date(phone.last_sync_at*1000).toLocaleString()}`:'Paired. Tap Sync now on the phone when you’re ready.'}</p>{!!phone.bytes_transferred&&status.running&&<small>{size(phone.bytes_transferred)} transferred this session</small>}</div>{forget===phone.id?<div className="wifi-actions"><button onClick={()=>{void operate('revoke',phone.id);setForget(null);}}>Confirm forget</button><button onClick={()=>setForget(null)}>Cancel</button></div>:<div className="wifi-actions"><button disabled={working||!status.running} aria-pressed={!!phone.listening_allowed} onClick={()=>void operate(phone.listening_allowed?'block_listening':'allow_listening',phone.id)}>{phone.listening_allowed?'Turn off listening access':'Allow listening activity'}</button><button disabled={working||!status.running} aria-pressed={!!phone.remote_allowed} onClick={()=>void operate(phone.remote_allowed?'block_remote':'allow_remote',phone.id)}>{phone.remote_allowed?'Revoke Mac Remote':'Allow Mac Remote'}</button><button onClick={()=>setForget(phone.id)}>Forget…</button></div>}</article>):<p className="wifi-note">No phone paired yet. Your offline copies stay on the phone when you stop sharing or forget it.</p>}</section>
  {(error||preparation?.error||status?.error)&&<p role="alert" className="inline-error">{error||preparation?.error||status?.error}</p>}
  <details><summary>How copying works</summary><p className="wifi-note">Only approved phones can copy your selection over an encrypted connection. Downloads resume after interruption and are verified before saving. Vorbis creates separate cached copies; your original files and saved collection order stay intact. Actual size varies with the audio.</p></details>
  <details open={advanced} onToggle={e=>setAdvanced(e.currentTarget.open)}><summary>Embedded musicOS player tools</summary>{advanced&&<EmbeddedDevices busy={busy} onSync={onSync}/>}</details>
 </section>;
}
