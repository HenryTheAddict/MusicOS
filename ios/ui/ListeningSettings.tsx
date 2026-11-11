import {useEffect,useRef,useState} from 'react';
import {request} from './bridge';

type ListeningStatus = {
 state: 'unpaired' | 'unchecked' | 'connected' | 'approval' | 'update_mac' | 'offline';
 lastfm_ready: boolean;
 discord_ready: boolean;
 pending: number;
 storage_error: boolean;
};

const connectionHint: Record<ListeningStatus['state'], string> = {
 unpaired: 'Pair your Mac in Computer first.',
 unchecked: 'Check your Mac to see which services are available.',
 connected: 'Connected to your Mac.',
 approval: 'In Devices on your Mac, choose Allow listening activity for this iPhone.',
 update_mac: 'Update musicOS on your Mac to use listening sync.',
 offline: 'Your Mac is unavailable. Turn on Wi-Fi sharing in Devices and join the same network.',
};

export function ListeningSettings({paired,lastfm,discord,onChange}:{
 paired:boolean;lastfm:boolean;discord:boolean;
 onChange:(key:'lastfmListening'|'discordPresence',value:boolean)=>void;
}){
 const [status,setStatus]=useState<ListeningStatus>();
 const [checking,setChecking]=useState(false);
 const serial=useRef(0);
 async function readStatus(refresh=false){
  const revision=++serial.current;
  if(refresh)setChecking(true);
  try {
   const result:ListeningStatus=await request({action:'listening_status',refresh});
   if(revision===serial.current)setStatus(result);
  } catch {
   if(revision===serial.current&&refresh)setStatus(current=>({...current,lastfm_ready:false,discord_ready:false,pending:current?.pending??0,storage_error:false,state:'offline'}));
  } finally {
   if(refresh&&revision===serial.current)setChecking(false);
  }
 }
 useEffect(()=>{
  void readStatus(paired);
  return ()=>{++serial.current;};
 },[paired]);
 useEffect(()=>{
  if(!paired||checking)return;
  const timer=window.setInterval(()=>{
   if(!document.hidden)void readStatus();
  },2000);
  return ()=>clearInterval(timer);
 },[paired,checking]);
 const state=paired?(status?.state??'unchecked'):'unpaired';
 return <section className="phone-settings-group phone-listening">
  <h2>Listening sync</h2>
  <div className="phone-listening-status" role="status">
   <p>{checking?'Checking your Mac…':connectionHint[state]}</p>
   {!!status?.pending&&<small>{status.pending} {status.pending===1?'listen':'listens'} waiting to send to your Mac</small>}
   {status?.storage_error&&<small>Couldn’t save the listening queue. Check available storage.</small>}
  </div>
  <label className="phone-setting-row"><span><strong>Last.fm listening history</strong>
   <small>Eligible listens are saved offline and sent to the Last.fm account connected on your Mac. Turning this off clears unsent phone listens.</small>
   {state==='connected'&&!status?.lastfm_ready&&<small>Enable Last.fm in Settings → Integrations on the Mac.</small>}
  </span><input type="checkbox" role="switch" checked={lastfm} disabled={!paired&&!lastfm} onChange={()=>onChange('lastfmListening',!lastfm)}/></label>
  <label className="phone-setting-row"><span><strong>Discord presence</strong>
   <small>Shares the song you’re playing through Discord or Vesktop on the Mac while both devices are connected.</small>
   {state==='connected'&&!status?.discord_ready&&<small>Enable Discord presence in Settings → Integrations on the Mac.</small>}
  </span><input type="checkbox" role="switch" checked={discord} disabled={!paired&&!discord} onChange={()=>onChange('discordPresence',!discord)}/></label>
  {paired&&<button className="filled compact" disabled={checking} onClick={()=>void readStatus(true)}>Check connection</button>}
 </section>;
}
