import './sparkSetup.css';
import {installSparkModel} from './sparkModel';
import {displayData,peekDisplayData,invalidateDisplay} from './displayData';
import {useViewActive} from './ViewScope';
import ThemedSwitch from './ThemedSwitch';
import {Icon} from './components';
import {product} from './product';
import {useEffect,useState,useRef} from 'react';
import {open} from '@tauri-apps/plugin-dialog';
import {call,errorText,native} from './bridge';
import type {SparkRequest,SparkState} from './types';
export function sparkSetupStatus(enabled:boolean,state:SparkState|null,error:string){return !enabled?'Beta off':error||state?.installation?.error?'Needs attention':state?.installation?.running?'Installing':!state?'Checking setup':!state.ready?'Setup needed':state.analyzed<2||state.analysis?.running?'Preparing songs':'Ready';}
export default function SparkSetup({enabled,onChange}:{enabled:boolean;onChange:(enabled:boolean)=>void}){
 const active=useViewActive();
 const [state,setState]=useState<SparkState|null>(()=>peekDisplayData('sparklist',{request:{action:'status'}})??null),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 const previouslyReady=useRef(!!state?.ready);
 const refreshRef=useRef<()=>Promise<void>>(async()=>{});
 const working=!!state?.installation?.running||!!state?.analysis?.running&&!state.analysis.paused;
 useEffect(()=>{if(!active)return;let alive=true,pending=false;
  async function refresh(){if(pending)return;pending=true;try{const next=await displayData('sparklist',{request:{action:'status'}},true);if(alive){setState(next);setError('');if(next.ready&&!previouslyReady.current){window.dispatchEvent(new Event('musicos-spark-ready'));invalidateDisplay('playlist_suggestions');invalidateDisplay('spark_recommendations');}previouslyReady.current=next.ready;}}catch(e){if(alive)setError(errorText(e));}finally{pending=false;}}
  refreshRef.current=refresh;void refresh();
  const changed=(event:Event)=>{const reason=(event as CustomEvent).detail;if(event.type==='musicos-library-changed'||!reason||reason==='sparklist')void refresh();};
  window.addEventListener('musicos-display-invalidated',changed);window.addEventListener('musicos-library-changed',changed);
  return()=>{alive=false;window.removeEventListener('musicos-display-invalidated',changed);window.removeEventListener('musicos-library-changed',changed);};
 },[enabled,active]);
 useEffect(()=>{if(!active||!working)return;const timer=setInterval(()=>void refreshRef.current(),1500);return()=>clearInterval(timer);},[active,working]);
 async function run(request:SparkRequest){if(request.action==='remove_model')autoAttempt.current=true;setBusy(true);setError('');try{setState(await call('sparklist',{request}));if(request.action==='remove_model')onChange(false);invalidateDisplay('sparklist');}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 async function install(){if(state?.staged_available){setBusy(true);setError('');try{setState(await installSparkModel());}catch(e){setError(errorText(e));}finally{setBusy(false);}return;}try{const directory=await open({directory:true,multiple:false,title:'Choose the verified Sparklist model folder'});if(typeof directory==='string')await run({action:'install',directory});}catch(e){setError(errorText(e));}}
 const autoAttempt=useRef(false);
 useEffect(()=>{if(!enabled){autoAttempt.current=false;return;}if(!active||!state||state.beta===false||state.ready||state.installed||state.installation?.running||state.installation?.error||!state.staged_available||busy||autoAttempt.current)return;autoAttempt.current=true;void install();},[active,enabled,state,busy]);
 const installing=!!state?.installation?.running,status=sparkSetupStatus(enabled,state,error);
 const detail=error||state?.installation?.error||state?.reason;
 const available=state?.available??0,prepared=state?.analyzed??0,analysis=state?.analysis;
 const progress=installing?Math.min(1,(state?.installation?.copied??0)/Math.max(1,state?.installation?.total??1)):available?Math.min(1,prepared/available):0;
 async function prepare(action:'start'|'resume'|'pause'){setBusy(true);setError('');try{await call('audio_analysis',{action});invalidateDisplay('sparklist');}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <article className="settings-tile spark-setup" data-enabled={enabled} data-state={status}>
  <header className="spark-setup-header"><div><span className="spark-setup-label"><Icon name="Sparklist"/>Audio mixes</span><h2>Sparklist beta</h2></div><span className="setup-status" role="status">{status}</span></header>
  <div className="spark-setup-layout">
   <div className="spark-setup-art" aria-hidden="true"><div className="spark-setup-cover"><span/><span/><Icon name="Sparklist"/></div><div className="spark-setup-art-label"><Icon name="Song"/><span>From your library</span></div></div>
   <div className="spark-setup-main">
    <ThemedSwitch label="Audio Sparklists" ariaLabel="Experimental audio Sparklist beta" checked={enabled} onChange={onChange}/>
    <p className="spark-setup-description">{enabled?'Prepare your songs once, then create mixes from a song or a mood.':state?.ready?'Your model and prepared songs are kept while this is off.':'Enable audio Sparklists to set up the included model. Your existing mixes stay as they are.'}</p>
    <ol className="spark-setup-stages" aria-label="Sparklist setup">
     <li data-ready={!!state?.ready}><span className="spark-stage-icon"><Icon name={state?.ready?'Downloaded':'Download'}/></span><div><strong>{installing?'Installing model':state?.ready?'Model installed':'Local model'}</strong><small>{installing?`${((state?.installation?.copied??0)/1_000_000).toFixed(1)} / ${((state?.installation?.total??0)/1_000_000).toFixed(1)} MB`:state?.ready?'Verified and stored on this device.':`${((state?.download_bytes??281234043)/1_000_000).toFixed(0)} MB · included with ${product.name}`}</small></div></li>
     <li data-ready={prepared>=2}><span className="spark-stage-icon"><Icon name="Song"/></span><div><strong>{analysis?.paused?'Preparation paused':analysis?.running?'Preparing songs':'Song preparation'}</strong><small>{state?`${prepared} of ${available} songs prepared.`:'Checking your library…'}</small></div>{available>0&&<span className="spark-setup-count">{Math.round(prepared/available*100)}%</span>}</li>
    </ol>
    {enabled&&(installing||analysis?.running||prepared>0)&&<div className="spark-setup-progress"><progress aria-label={installing?'Model installation':'Song preparation'} max={1} value={progress}/>{analysis?.running&&<small role="status">{analysis.paused?'Paused':'Preparing songs'} · {analysis.processed} / {analysis.total}</small>}</div>}
    {enabled&&<div className="spark-setup-actions">{installing?<button className="soft-action" disabled={busy} onClick={()=>void run({action:'cancel_install'})}>Cancel installation</button>:!state?.ready?<button className="spark-primary" disabled={busy||!native} onClick={()=>void install()}><Icon name="Download"/>{status==='Needs attention'?'Retry installation':state?.staged_available?'Install included model':'Choose model folder'}</button>:<button className="spark-primary" disabled={busy||!native} onClick={()=>void prepare(analysis?.running&&!analysis.paused?'pause':analysis?.paused?'resume':'start')}><Icon name={analysis?.running&&!analysis.paused?'pause':'Sparklist'}/>{analysis?.running&&!analysis.paused?'Pause preparation':analysis?.paused?'Resume preparation':'Prepare songs'}</button>}<small>{installing?'Playback stays available.':prepared>=2?'Prepared songs are ready to use in a Sparklist.':'At least two prepared songs are needed for a mix.'}</small></div>}
    {status==='Needs attention'&&<p className="spark-setup-error" role="alert">{error||state?.installation?.error||'Setup needs attention. Open details for more information.'}</p>}
   </div>
  </div>
  <details className="spark-setup-details"><summary><Icon name="Back"/><span>Model, storage & details</span></summary><div><p>LAION larger_clap_general · Apache 2.0 · {((state?.download_bytes??281234043)/1_000_000).toFixed(1)} MB. Included with musicOS and verified before installation.</p><p>Experimental model; listening evaluation is still in progress. Prepared-song coverage does not measure recommendation quality.</p>{detail&&<p>{detail}</p>}{!state?.ready&&!state?.staged_available&&<button className="soft-action" disabled={busy||!native} onClick={()=>void install()}>Choose verified model folder…</button>}{(state?.installed||state?.ready)&&<button className="soft-action" disabled={busy||installing} onClick={()=>void run({action:'remove_model'})}>Remove model</button>}</div></details>
 </article>;
}
