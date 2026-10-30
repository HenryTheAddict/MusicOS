import {useEffect,useRef,useState} from 'react';
import {beatOffsets} from './heartVisibility';
import {call} from './bridge';

// Tempo-only responses keep beat arrays and analysis diagnostics off the animation path.
export default function useAnalyzedTempos(enabled:boolean,libraryIdentity:string,playingId?:number){
 const [tempos,setTempos]=useState<ReadonlyMap<number,number>>(()=>new Map());
 const refresh=useRef<(()=>void)|undefined>(undefined),latest=useRef(tempos),lastPlaying=useRef(playingId);latest.current=tempos;
 useEffect(()=>{
  if(!enabled)return;
  let alive=true,pending=false,again=false,failures=0,timer:ReturnType<typeof setTimeout>|undefined;
  async function update(){
   if(document.hidden)return;
   if(pending){again=true;return;}
   clearTimeout(timer);pending=true;
   try{
    const status=await call('audio_tempos');
    if(!alive)return;
    failures=0;
    const next=new Map<number,number>();beatOffsets.clear();for(const feature of status.tempos)if(Number.isFinite(feature.beat_offset))beatOffsets.set(feature.track_id,feature.beat_offset!);
    for(const feature of status.tempos)if(Number.isFinite(feature.bpm)&&feature.bpm>=20&&feature.bpm<=400)next.set(feature.track_id,feature.bpm);
    setTempos(old=>old.size===next.size&&[...next].every(([id,bpm])=>old.get(id)===bpm)?old:next);
    if(status.running)timer=setTimeout(()=>void update(),5000);
   }catch{
    // Keep valid cached tempos through transient busy/offline responses.
    if(alive&&failures<3)timer=setTimeout(()=>void update(),[1000,2500,5000][failures++]);
   }finally{pending=false;if(alive&&again){again=false;void update();}}
  }
  const request=()=>{failures=0;void update();};refresh.current=request;
  const visible=()=>{if(!document.hidden)request();else clearTimeout(timer);};
  const libraryChanged=(event:Event)=>{if((event as CustomEvent<{reason?:string}>).detail?.reason==='analysis')request();};
  void update();
  window.addEventListener('musicos-audio-rescanned',request);
  window.addEventListener('musicos-audio-analysis-changed',request);
  window.addEventListener('musicos-library-changed',libraryChanged);
  document.addEventListener('visibilitychange',visible);
  return()=>{alive=false;refresh.current=undefined;clearTimeout(timer);window.removeEventListener('musicos-audio-rescanned',request);window.removeEventListener('musicos-audio-analysis-changed',request);window.removeEventListener('musicos-library-changed',libraryChanged);document.removeEventListener('visibilitychange',visible);};
 },[enabled,libraryIdentity]);
 useEffect(()=>{const changed=lastPlaying.current!==playingId;lastPlaying.current=playingId;if(changed&&enabled&&playingId!==undefined&&!latest.current.has(playingId))refresh.current?.();},[enabled,playingId]);
 return tempos;
}
