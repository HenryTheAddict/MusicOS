import { useEffect, useState, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { call, errorText, native } from "./bridge";
import { Icon } from "./components";
import "./devicesWifi.css";
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

export interface WifiStatus {
 running:boolean;name?:string;track_count:number;bytes_total:number;skipped:number;selected_ids:number[];
 peers:{id:string;name:string;last_sync_at:number|null;track_count:number;bytes_transferred?:number}[];
 pending:{id:string;name:string;created_at:number}[];
 pairing:{link:string;qr:string;expires_at:number}|null;error:string|null;
}
const size=(bytes:number)=>bytes>=1073741824?`${(bytes/1073741824).toFixed(1)} GB`:`${(bytes/1048576).toFixed(1)} MB`;
export default function Devices({library,collections,onSync,busy}:{library:Library;collections:SavedCollection[];onSync:(root:string)=>void;busy:boolean}){
 const [status,setStatus]=useState<WifiStatus|null>(null),[selection,setSelection]=useState('all'),[working,setWorking]=useState(false),[error,setError]=useState(''),[copied,setCopied]=useState(false),[advanced,setAdvanced]=useState(false),[forget,setForget]=useState<string|null>(null);
 const operation=useRef(0);
 const ids=selection==='all'?library.tracks.filter(t=>t.available).map(t=>t.id):selection==='favorites'?library.tracks.filter(t=>t.favorited&&t.available).map(t=>t.id):collections.find(c=>c.id===selection)?.track_ids??[];
 async function operate(action:string,peerId?:string){const request=++operation.current;setError('');setWorking(true);try{const result=await call('wifi_sync',{action,peerId,trackIds:action==='start'?[...new Set(ids)]:undefined});if(request===operation.current){setStatus(result);if(action==='pair')setCopied(false);}}catch(e){if(request===operation.current)setError(errorText(e));}finally{if(request===operation.current)setWorking(false);}}
 useEffect(()=>{let gone=false,inFlight=false;async function refresh(){if(inFlight)return;inFlight=true;try{const next=await call('wifi_sync',{action:'status'});if(!gone)setStatus(next);}catch(e){if(!gone)setError(errorText(e));}finally{inFlight=false;}}void refresh();const timer=setInterval(()=>{if(!document.hidden)void refresh();},2000);return()=>{gone=true;clearInterval(timer);};},[]);
 return <section className="wifi-devices">
  <header><div><h1>Devices</h1><p>Your music on your phone, even offline.</p></div><span className="wifi-state">{status?.running?'Sharing on this network':'Wi-Fi sharing is off'}</span></header>
  <section className="wifi-share"><div><h2>Music to copy</h2><p>Choose songs or a collection. Originals and saved sequences stay on this Mac.</p></div><label>Selection<ThemedChoice label="Music to copy" value={selection} disabled={working} onChange={setSelection} options={[{value:"all",label:"Available library",icon:"Library"},{value:"favorites",label:"Favorites",icon:"hearted"},...collections.map(c=>({value:c.id,label:c.name}))]}/></label>
   <div className="wifi-actions"><span>{new Set(ids).size} songs selected</span><button data-playful className="primary" disabled={working||!ids.length} onClick={()=>void operate('start')}><Icon name="sync"/>{working?'Preparing verified copies…':status?.running?'Update shared music':'Start Wi-Fi sharing'}</button>{(status?.running||working)&&<button onClick={()=>void operate('stop')}>Stop sharing</button>}</div>
   {status?.running&&<p className="wifi-note">{status.track_count} songs · {size(status.bytes_total)} ready. Keep musicOS open while the phone copies them.{status.skipped>0?` ${status.skipped} unavailable or unsupported files were skipped.`:''}</p>}
  </section>
  <section className="wifi-phone"><header><div><h2>Pair your Android phone</h2><p>Connect both devices to the same Wi-Fi, then open musicOS on Android.</p></div><button data-playful disabled={working||!status?.running} onClick={()=>void operate('pair')}><Icon name="Plus"/>{status?.pairing?'New pairing link':'Create pairing link'}</button></header>
   {status?.pairing&&<div className="wifi-pair"><img alt="Pairing QR code" src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(status.pairing.qr)}`}/><div><strong>Open this link on your phone</strong><p>Scan with your phone’s camera, or copy the link into Devices in the Android app. The link expires after two minutes. Approve the matching request here.</p><button onClick={()=>void navigator.clipboard.writeText(status.pairing!.link).then(()=>setCopied(true)).catch(()=>setError('Could not copy the pairing link. Try scanning the QR code.'))}>{copied?'Copied':'Copy pairing link'}</button></div></div>}
   {!status?.running&&<p className="wifi-note">Start sharing to pair a phone. Sharing stays off when musicOS launches.</p>}
   {status?.pending.map(phone=><article className="wifi-request" key={phone.id}><div><strong>{phone.name}</strong><small>Request {phone.id.slice(0,6)} · compare with the phone</small></div><button disabled={working} className="primary" onClick={()=>void operate('approve',phone.id)}>Approve</button><button disabled={working} onClick={()=>void operate('reject',phone.id)}>Decline</button></article>)}
  </section>
  <section className="wifi-paired"><h2>Paired phones</h2>{status?.peers.length?status.peers.map(phone=><article key={phone.id}><Icon name="device"/><div><strong>{phone.name}</strong><p>{phone.last_sync_at?`${phone.track_count} songs copied · ${new Date(phone.last_sync_at*1000).toLocaleString()}`:'Ready for the first copy. Choose Copy music on the phone.'}</p>{!!phone.bytes_transferred&&status.running&&<small>{size(phone.bytes_transferred)} transferred this session</small>}</div>{forget===phone.id?<div className="wifi-actions"><button onClick={()=>{void operate('revoke',phone.id);setForget(null);}}>Confirm forget</button><button onClick={()=>setForget(null)}>Cancel</button></div>:<button onClick={()=>setForget(phone.id)}>Forget…</button>}</article>):<p className="wifi-note">Your paired phones will appear here. Forgetting a phone revokes its access and keeps its offline copies.</p>}</section>
  {(error||status?.error)&&<p role="alert" className="inline-error">{error||status?.error}</p>}
  <p className="wifi-note">Transfers use an encrypted, pinned connection. Only approved phones can read selected music. Copies are verified before appearing in the phone’s library; interrupted downloads resume. Keep originals in formats your Android device supports.</p>
  <details open={advanced} onToggle={e=>setAdvanced(e.currentTarget.open)}><summary>Embedded musicOS player tools</summary>{advanced&&<EmbeddedDevices busy={busy} onSync={onSync}/>}</details>
 </section>;
}
