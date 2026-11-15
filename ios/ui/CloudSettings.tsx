import {useEffect,useState} from 'react';
import {AssetIcon as Icon} from '../../ui-desktop/src/AssetIcon';
import {request,errorText} from './bridge';

type LibraryCopy={device:string;name:string;songs:number;updated:number;current:boolean};
type CloudStatus={
 phase:'idle'|'checking'|'saving'|'importing'|'error';
 automatic:boolean;connected:boolean;folder?:string;last_saved?:number;
 copies:LibraryCopy[];song_count:number;message?:string;
};

const working:Record<string,string>={checking:'Checking iCloud Drive…',saving:'Saving library files…',importing:'Downloading and verifying files…'};

export default function CloudSettings(){
 const [open,setOpen]=useState(false),[state,setState]=useState<CloudStatus>(),[pending,setPending]=useState(false);
 const [error,setError]=useState(''),[selected,setSelected]=useState<LibraryCopy>();
 const busy=!!state&&!!working[state.phase];
 useEffect(()=>{
  if(!open)return;
  let active=true;
  const read=()=>request({action:'cloud_status'}).then((status:CloudStatus)=>{if(active)setState(status);})
   .catch(e=>{if(active)setError(errorText(e));});
  void read();
  const interval=busy?window.setInterval(()=>{if(!document.hidden)void read();},900):undefined;
  return()=>{active=false;clearInterval(interval);};
 },[open,busy]);

 async function run(action:string,args:Record<string,unknown>={}){
  if(pending)return;
  setPending(true);setError('');
  try{setState(await request({action,...args}));}catch(e){setError(errorText(e));}
  finally{setPending(false);}
 }

 return <details className="phone-cloud" onToggle={event=>setOpen(event.currentTarget.open)}>
  <summary><Icon name="Storage"/><span><strong>iCloud Drive</strong><small>Sync music and library copies</small></span><Icon name="Back"/></summary>
  {open&&<div className="phone-cloud-body">
   {!state?.connected?<><p>Choose a folder in iCloud Drive for your downloaded songs, playlists, favorites, cover art, and lyrics. On another iPhone, choose the same folder to bring a copy into musicOS.</p>
    <button className="phone-cloud-primary" disabled={pending} onClick={()=>void run('cloud_choose')}>Choose iCloud Drive folder</button>
   </>:<>
    <p><strong>{state.folder}</strong> · {state.song_count} downloaded {state.song_count===1?'song':'songs'}</p>
    {state.last_saved!==undefined&&<small>Last saved {new Date(state.last_saved*1000).toLocaleString()}</small>}
    <label className="phone-setting-row"><span><strong>Save in background</strong><small>Refresh this iPhone’s copy when iOS schedules library sync.</small></span>
     <input type="checkbox" role="switch" checked={state.automatic} disabled={pending||busy} onChange={e=>void run('cloud_auto',{enabled:e.target.checked})}/></label>
    <div className="phone-cloud-actions">
     <button className="phone-cloud-primary" disabled={pending||busy||!state.song_count} onClick={()=>void run('cloud_upload')}>Save library now</button>
     <button disabled={pending||busy} onClick={()=>void run('cloud_list')}>Find library copies</button>
     {busy&&<button disabled={pending} onClick={()=>void run('cloud_cancel')}>Cancel transfer</button>}
    </div>
    {state.copies.length>0&&<ul>{state.copies.map(copy=><li key={copy.device}>
     <span><strong>{copy.current?'This iPhone':copy.name}</strong><small>{copy.songs} songs · {new Date(copy.updated*1000).toLocaleString()}</small></span>
     <button disabled={pending||busy} onClick={()=>setSelected(copy)}>Review</button>
    </li>)}</ul>}
    {selected&&<div className="phone-cloud-review" role="region" aria-label="Review iCloud library copy">
     <strong>Merge {selected.songs} songs from {selected.current?'this iPhone’s copy':selected.name}?</strong>
     <p>Existing songs and favorites are kept. Different playlist versions are added as separate copies.</p>
     <div className="phone-cloud-actions"><button disabled={pending||busy} className="phone-cloud-primary" onClick={()=>{
      void run('cloud_import',{device:selected.device,updated:selected.updated});setSelected(undefined);
     }}>Merge library copy</button><button onClick={()=>setSelected(undefined)}>Back</button></div>
    </div>}
    <button disabled={pending||busy} onClick={()=>{setSelected(undefined);void run('cloud_disconnect');}}>Disconnect folder</button>
   </>}
   {(busy||state?.message)&&<p role="status">{busy?working[state!.phase]:state?.message}</p>}
   {error&&<p role="alert">{error}</p>}
  </div>}
 </details>;
}
