import {usePlaybackClock} from './playbackFeed';
import {memo,useEffect,useLayoutEffect,useRef,useState,type CSSProperties,type PointerEvent} from 'react';
import type {Playback,QueueMix} from './types';
import {duration as formatTime} from './components';
import {animateOwned,cancelOwned} from './presentationMotion';
import {call} from './bridge';
import './waveform.css';
type Drag={pointerId:number;left:number;width:number;maximum:number};
type Wave={low:number[];mid:number[];high:number[]};
const waveRequests=new Map<number,Promise<Wave>>();
function readWave(id:number){
 let work=waveRequests.get(id);
 if(!work){
  work=call('track_waveform',{trackId:id});
  waveRequests.set(id,work);
  if(waveRequests.size>24)waveRequests.delete(waveRequests.keys().next().value!);
  void work.catch(()=>{if(waveRequests.get(id)===work)waveRequests.delete(id);});
 }
 return work;
}

const SeekWave = memo(function SeekWave({wave}:{wave:Wave}){
 const count=wave.low.length;
 if(count<2)return null;
 // An unbroken envelope keeps its shape on wide rails and thin header players.
 // Bass sits at the center, with mid and high energy contributing to the silhouette.
 const energy=wave.low.map((value,index)=>Math.hypot(value*.78,wave.mid[index]*.58,wave.high[index]*.44));
 const peak=Math.max(1,...energy);
 const envelope=(height:(index:number)=>number)=>{
  const points=energy.map((_,index)=>[index*640/(count-1),height(index)]);
  const top=points.map(([x,y])=>`${x.toFixed(1)},${(16-y).toFixed(1)}`).join(' ');
  const bottom=points.reverse().map(([x,y])=>`${x.toFixed(1)},${(16+y).toFixed(1)}`).join(' ');
  return `M${top} L${bottom} Z`;
 };
 const outer=envelope(index=>.35+energy[index]/peak*14.4);
 const inner=envelope(index=>energy[index]/peak*(3.1+wave.low[index]/255*5.5));
 return <svg aria-hidden="true" className="seek-wave" viewBox="0 0 640 32" preserveAspectRatio="none">
  <path className="seek-wave-envelope" d={outer}/>
  <path className="seek-wave-body" d={inner}/>
 </svg>;
});

function mixWindow(entryId:number|undefined, duration:number, transition:Playback['dj_transition'], plan:QueueMix|undefined){
 if(entryId===undefined||duration<=0)return;
 const live=transition&&(transition.outgoing_entry===entryId||
  (transition.active&&transition.incoming_entry===entryId))?transition:undefined;
 const incoming=live?.active&&live.incoming_entry===entryId;
 const forecast=plan?.outgoing_entry===entryId?plan:undefined;
 const cue=live?(incoming?live.incoming_cue:live.outgoing_cue):forecast?.outgoing_cue;
 const span=live?(live.source_seconds??live.seconds)*(incoming?live.tempo:1):forecast?.seconds;
 if(cue===undefined||span===undefined||!Number.isFinite(cue)||!Number.isFinite(span)||span<=0||cue>=duration)return;
 const start=Math.max(0,cue);
 const end=Math.min(duration,cue+span);
 if(end<=start)return;
 const state=live?.active?'active':live?.status??'planned';
 return {left:start/duration*100,width:(end-start)/duration*100,state,
  description:`AutoMix ${state}, ${formatTime(start)} to ${formatTime(end)}`};
}

export default function PlaybackProgress({trackId,entryId,position,duration,seek,showTime=true,revealTime=false,transition,plannedMix,live=true,roundedFill=false,djMode=false}:{revealTime?:boolean;live?:boolean;roundedFill?:boolean;showTime?:boolean;transition?:Playback['dj_transition'];plannedMix?:QueueMix;trackId?:number;entryId?:number;position:number;duration:number;seek:(position:number)=>Promise<void>;djMode?:boolean}){
 const clock=usePlaybackClock(live);if(live&&clock.trackId===trackId)position=clock.position;
 const [draft,setDraft]=useState<number|null>(null),[pending,setPending]=useState(false),[scrubbing,setScrubbing]=useState(false),[error,setError]=useState('');
 const [wave,setWave]=useState<{id:number;data:Wave}>();
 useEffect(()=>{
  if(!djMode||trackId===undefined)return;
  let current=true;
  const delay=window.setTimeout(()=>{
   void readWave(trackId).then(data=>{if(current)setWave({id:trackId,data});}).catch(()=>{});
  },120);
  return()=>{current=false;window.clearTimeout(delay);};
 },[djMode,trackId]);
 const waveform=djMode&&wave?.id===trackId?wave?.data:undefined;
 const input=useRef<HTMLInputElement>(null),host=useRef<HTMLDivElement>(null),slot=useRef<HTMLDivElement>(null),rail=useRef<HTMLDivElement>(null),time=useRef<HTMLOutputElement>(null),drag=useRef<Drag|null>(null),operation=useRef(0);
 const identity=`${trackId??''}:${entryId??''}`,last=useRef(identity);
 const maximum=Math.max(0,Number.isFinite(duration)?duration:0),value=Math.max(0,Math.min(maximum,draft??position)),fraction=maximum>0?value/maximum:0;
 const blend=djMode?mixWindow(entryId,maximum,transition,plannedMix):undefined;
 function release(){const pointer=drag.current;drag.current=null;setScrubbing(false);slot.current?.style.removeProperty('--frozen-rail-width');if(pointer&&input.current?.hasPointerCapture?.(pointer.pointerId))input.current.releasePointerCapture(pointer.pointerId);}
 function cancel(){release();setDraft(null);}
 useLayoutEffect(()=>{
  operation.current++;cancel();setPending(false);setError('');
  const previous=last.current;last.current=identity;
  if(!roundedFill&&previous!==identity&&previous.split(':')[0]&&trackId&&rail.current)animateOwned(rail.current,'song-progress-handoff',[{clipPath:'inset(0 100% 0 0)',opacity:.4},{clipPath:'inset(0 0% 0 0)',opacity:1}],180);
 },[identity]);
 useLayoutEffect(()=>()=>{operation.current++;if(rail.current)cancelOwned(rail.current);},[]);
 useLayoutEffect(()=>{
  const label=time.current,node=host.current;if(!revealTime||!showTime||!trackId||!label||!node)return;
  const measure=()=>{const width=`${Math.ceil(label.getBoundingClientRect().width)+8}px`;if(node.style.getPropertyValue('--time-reserve')!==width)node.style.setProperty('--time-reserve',width);};
  measure();const observer=new ResizeObserver(measure);observer.observe(label);return()=>observer.disconnect();
 },[revealTime,showTime,trackId]);
 async function commit(next:number){
  const revision=++operation.current;setPending(true);setError('');
  try{await seek(Math.max(0,Math.min(maximum,next)));}catch{if(revision===operation.current)setError('Could not seek. Try again.');}
  finally{if(revision===operation.current){setPending(false);setDraft(null);}}
 }
 function pointerValue(event:PointerEvent<HTMLInputElement>){const held=drag.current;return held&&held.width>0&&Number.isFinite(event.clientX)?Math.max(0,Math.min(held.maximum,(event.clientX-held.left)/held.width*held.maximum)):Number(event.currentTarget.value);}
 return <div ref={host} className="playback-progress" data-reveal-time={revealTime&&showTime&&!!trackId||undefined} data-seeking={pending||scrubbing||undefined} data-scrubbing={scrubbing||undefined}>
  <div ref={slot} className="playback-rail-slot" data-waveform={waveform?true:undefined}><div ref={rail} className="progress-rail" aria-hidden="true">{waveform?<><SeekWave wave={waveform}/><i className="seek-wave-position" style={{transform:`translateX(${fraction*100}%)`}}/></>:<span style={roundedFill?{width:`${fraction*100}%`}:{width:'100%',transform:`scaleX(${fraction})`}}/>}</div>
   {blend&&<div className="transition-region" aria-hidden="true" data-state={blend.state}
    style={{left:`${blend.left}%`,width:`${blend.width}%`}}/>}
   <input ref={input} className="seek" type="range" aria-label="Playback position" aria-description={blend?.description} aria-valuetext={`${formatTime(value)} of ${formatTime(maximum)}`} min={0} max={Math.max(1,maximum)} step={.1} value={value} disabled={!trackId||maximum===0} style={{'--progress':`${fraction*100}%`} as CSSProperties}
    onPointerDown={event=>{
     if(event.button!==undefined&&event.button!==0)return;
     const box=event.currentTarget.getBoundingClientRect();drag.current={pointerId:event.pointerId,left:box.left,width:box.width,maximum};
     if(box.width>0){event.preventDefault();slot.current?.style.setProperty('--frozen-rail-width',`${box.width}px`);event.currentTarget.focus({preventScroll:true});event.currentTarget.setPointerCapture?.(event.pointerId);}
     setScrubbing(true);setDraft(pointerValue(event));
    }}
    onPointerMove={event=>{if(drag.current&&drag.current.pointerId===event.pointerId)setDraft(pointerValue(event));}}
    onChange={event=>{const next=Number(event.currentTarget.value);setDraft(next);if(!drag.current)void commit(next);}}
    onPointerUp={event=>{if(drag.current&&drag.current.pointerId===event.pointerId){const next=pointerValue(event);release();void commit(next);}}}
    onPointerCancel={cancel} onLostPointerCapture={()=>{if(drag.current)cancel();}} onBlur={()=>{if(drag.current)cancel();}} onKeyDown={event=>{if(event.key==='Escape'&&drag.current){event.preventDefault();event.stopPropagation();cancel();}}}
   />
  </div>
  {showTime&&trackId&&<output ref={time} className="playback-time" aria-label={revealTime?`${formatTime(maximum-value)} remaining`:undefined}>{revealTime?<span className="remaining-time">−{formatTime(maximum-value)}</span>:<>{formatTime(value)} <span>/ {formatTime(maximum)}</span></>}</output>}
  {error&&<span className="seek-error" role="alert">{error}</span>}
 </div>;
}
