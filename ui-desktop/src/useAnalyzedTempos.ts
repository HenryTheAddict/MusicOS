import {useEffect,useState} from 'react';
import {call} from './bridge';

// One cached-analysis read for the library, never a request per song row.
export default function useAnalyzedTempos(enabled:boolean,libraryIdentity:string){
 const [tempos,setTempos]=useState<ReadonlyMap<number,number>>(()=>new Map());
 useEffect(()=>{
  if(!enabled)return;
  let alive=true,pending=false,again=false,timer:ReturnType<typeof setTimeout>|undefined;
  async function update(){
   if(pending){again=true;return;}
   clearTimeout(timer);pending=true;
   try{
    const status=await call('audio_analysis',{action:'status'});
    if(!alive)return;
    const next=new Map<number,number>();
    for(const feature of status.features)if(feature.bpm!==null&&Number.isFinite(feature.bpm)&&feature.bpm>=20&&feature.bpm<=400)next.set(feature.track_id,feature.bpm);
    setTempos(old=>old.size===next.size&&[...next].every(([id,bpm])=>old.get(id)===bpm)?old:next);
    if(status.running)timer=setTimeout(()=>void update(),5000);
   }catch{/* Missing analysis leaves the ordinary heart available. */}
   finally{pending=false;if(alive&&again){again=false;void update();}}
  }
  const refresh=()=>void update();
  void update();
  window.addEventListener('musicos-audio-rescanned',refresh);
  window.addEventListener('musicos-audio-analysis-changed',refresh);
  return()=>{alive=false;clearTimeout(timer);window.removeEventListener('musicos-audio-rescanned',refresh);window.removeEventListener('musicos-audio-analysis-changed',refresh);};
 },[enabled,libraryIdentity]);
 return tempos;
}
