import {displayData,invalidateDisplay} from './displayData';
import {useViewActive} from './ViewScope';
import {useEffect,useState} from 'react';
import {call,errorText,native} from './bridge';
import {AssetIcon} from './AssetIcon';
import ThemedSwitch from './ThemedSwitch';
import type {DepthStatus} from './types';
export default function DepthSetup({enabled,onChange}:{enabled:boolean;onChange:(enabled:boolean)=>void}){
 const active=useViewActive();
 const [statusEpoch,setStatusEpoch]=useState(0);
 const [state,setState]=useState<DepthStatus|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 useEffect(()=>{if(!active||!native)return;let alive=true,timer:ReturnType<typeof setTimeout>|undefined;
  const refresh=async()=>{try{const value=await displayData('depth_setup',{action:'status'},true);if(alive){setState(value);window.dispatchEvent(new CustomEvent('musicos-depth-ready',{detail:value.ready}));if(value.running)timer=setTimeout(()=>void refresh(),1500);}}catch(e){if(alive)setError(errorText(e));}};
  void refresh();return()=>{alive=false;clearTimeout(timer);};
 },[active,statusEpoch]);
 const run=async(action:string)=>{setBusy(true);setError('');try{const next=await call('depth_setup',{action});setState(next);invalidateDisplay('depth_setup');setStatusEpoch(n=>n+1);window.dispatchEvent(new CustomEvent('musicos-depth-ready',{detail:next.ready}));}catch(e){setError(errorText(e));}finally{setBusy(false);}};
 return <div className="depth-setup"><div className="depth-setup-heading"><AssetIcon name="depth"/><ThemedSwitch label="Depth artwork" checked={enabled} disabled={!native||state?.supported===false} onChange={onChange}/></div><p>Optional local depth for covers. Ordinary artwork stays visible while depth is prepared.</p>{native&&<div className="analysis-controls">{state?.running?<><progress aria-label="Depth model download" max={state.total} value={state.downloaded}/><button disabled={busy} onClick={()=>void run('cancel')}>Cancel download</button></>:state?.ready?<><span>Installed</span><button disabled={busy} onClick={()=>void run('remove')}>Remove depth model</button></>:<button className="sketch-button" disabled={busy||state?.supported===false} onClick={()=>void run('install')}>{state?.error?'Retry depth setup':'Download depth model'}</button>}</div>}{(error||state?.error)&&<p className="inline-error" role="alert">{error||state?.error}</p>}<details className="depth-model-details"><summary>Model details</summary><p>{state?.attribution??'Depth Anything V2 Small F32 · Apple Core ML · Apache 2.0'} · 99.2 MB</p><a href="https://huggingface.co/apple/coreml-depth-anything-v2-small" onClick={e=>{e.preventDefault();void call('open_provider',{url:e.currentTarget.href});}}>Model and attribution ↗</a></details></div>;
}
