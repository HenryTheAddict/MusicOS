import {useEffect,useRef,useState} from 'react';
import {AssetIcon as Icon} from '../../ui-desktop/src/AssetIcon';
import {errorText,request} from './bridge';

type Job={id:string;title:string;state:string;completed:number;total:number};
type Sync={state:string;message?:string;completed?:number;total?:number;time?:number};
type Activity={sync:Sync;history:Sync[];mac:Job[];following:string};
const activeSync=['starting','connecting','preparing','downloading','updating','uploading','cancelling'];
const title=(state:string)=>({running:'In progress',complete:'Complete',error:'Needs attention',failed:'Needs attention',cancelled:'Cancelled',interrupted:'Interrupted'}[state]??'In progress');

export default function PhoneActivity({selection,onClose,onComputer,liveSync}:{
 selection:string;onClose:()=>void;onComputer:()=>void;liveSync:boolean;
}){
 const [activity,setActivity]=useState<Activity>(),[error,setError]=useState(''),[pending,setPending]=useState(false);
 const target=useRef<HTMLElement>(null);
 useEffect(()=>{
  let live=true,timer=0;
  async function poll(){
   if(!document.hidden)try{const value=await request({action:'activity_jobs'});if(live)setActivity(value);}catch(error){if(live)setError(errorText(error));}
   if(live)timer=window.setTimeout(poll,1250);
  }
  void poll();
  return()=>{live=false;clearTimeout(timer);};
 },[]);
 useEffect(()=>{if(target.current){target.current.focus({preventScroll:true});target.current.scrollIntoView({block:'nearest'});}},[selection,!!activity]);
 async function run(action:string,id?:string){
  if(pending)return;
  setPending(true);setError('');
  try{await request({action,id});setActivity(await request({action:'activity_jobs'}));}
  catch(error){setError(errorText(error));}finally{setPending(false);}
 }
 const running=!!activity&&activeSync.includes(activity.sync.state);
 const sync=activity?.sync;
 return <section className="phone-sheet phone-activity phone-remote" role="dialog" aria-modal="true" aria-label="Activity">
  <header><h1>Activity</h1><button className="filled" onClick={onClose} aria-label="Close Activity"><Icon name="controlClose"/></button></header>
  {error&&<p role="alert">{error}</p>}
  <div className="remote-list">
   <h2>On this iPhone</h2>
   <article ref={selection==='sync'?target:undefined} tabIndex={-1}>
    <strong>Wi-Fi sync</strong><small>{sync?.message||'Copy music from your Mac for offline listening.'}</small>
    {running&&<progress max={sync?.total||1} value={sync?.total?sync.completed:undefined} aria-label="Wi-Fi sync progress"/>}
    {running?<button className="filled" disabled={pending||sync?.state==='cancelling'} onClick={()=>void run('cancel')}>Cancel</button>
     :<button className="filled" onClick={onComputer}>{sync?.state==='error'||sync?.state==='cancelled'?'Open sync to retry':'Open Computer'}</button>}
   </article>
   {!!activity?.mac.length&&<><h2>On your Mac</h2>{activity.mac.map(job=><article key={job.id} ref={selection===job.id?target:undefined} tabIndex={-1}>
    <strong>{job.title}</strong><small>{title(job.state)}</small>
    {job.state==='running'&&<progress max={job.total||1} value={job.total?job.completed:undefined} aria-label={job.title}/>}
    {job.state==='running'&&liveSync&&<button className="filled" disabled={pending} onClick={()=>void run('follow_activity',activity.following===job.id?undefined:job.id)}>
     {activity.following===job.id?'Stop following':'Show in Dynamic Island'}
    </button>}
   </article>)}</>}
   {!!activity?.history.length&&<><h2>Recent syncs</h2>{activity.history.map((job,index)=><article key={`${job.time}-${index}`}>
    <strong>{title(job.state)}</strong><small>{job.message}</small>
   </article>)}</>}
  </div>
 </section>;
}
