import {useEffect,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import {Icon} from './components';

export type ActivityJob={id:string;kind:string;title:string;state:string;completed:number;total:number;message:string;finished_at?:number};

export default function Activity({onClose,onResults}:{onClose:()=>void;onResults:(kind:string)=>void}){
 const dialog=useRef<HTMLDialogElement>(null);
 const [jobs,setJobs]=useState<ActivityJob[]>([]),[error,setError]=useState(''),[pending,setPending]=useState(false);
 useEffect(()=>{
  const opener=document.activeElement,element=dialog.current;
  element?.showModal();
  let active=true,timer=0;
  async function refresh(){
   try{const next=await call('activity_jobs');if(active){setJobs(next);setError('');}}
   catch(error){if(active)setError(errorText(error));}
   if(active)timer=window.setTimeout(refresh,1000);
  }
  void refresh();
  return()=>{active=false;clearTimeout(timer);element?.close();if(opener instanceof HTMLElement&&opener.isConnected)opener.focus({preventScroll:true});};
 },[]);
 async function cancel(kind:string){
  setPending(true);
  try{
   if(kind==='apple')await call('apple_enrichment',{request:{action:'stop_cache'}});
   else if(kind==='wifi')await call('wifi_sync',{action:'stop'});
   else await call('cancel_task');
   setJobs(await call('activity_jobs'));
  }catch(error){setError(errorText(error));}finally{setPending(false);}
 }
 return <dialog ref={dialog} className="song-selector activity-panel" aria-label="Activity" onCancel={event=>{event.preventDefault();onClose();}}>
  <header><h2>Activity</h2><button aria-label="Close Activity" onClick={onClose}><Icon name="controlClose"/></button></header>
  {error&&<p role="alert">{error}</p>}
  <div className="activity-list">{jobs.length?jobs.map(job=><article key={job.id}>
   <div><strong>{job.title}</strong><small>{job.state==='running'?'In progress':job.state==='complete'?'Complete':job.state==='cancelled'?'Cancelled':job.state==='interrupted'?'Interrupted':'Needs attention'}</small></div>
   {job.message&&<p>{job.message}</p>}
   {job.state==='running'&&<><progress aria-label={job.title} max={job.total||1} value={job.total?job.completed:undefined}/>
    <button disabled={pending} onClick={()=>void cancel(job.kind)}>Cancel</button></>}
   {job.state!=='running'&&['enrich','apple','wifi'].includes(job.kind)&&<button onClick={()=>onResults(job.kind)}>{job.state==='complete'?'Open results':'Open tools'}</button>}
  </article>):<p>No recent activity.</p>}</div>
 </dialog>;
}
