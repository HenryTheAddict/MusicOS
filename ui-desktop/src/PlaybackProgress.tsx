import {usePlaybackClock} from './playbackFeed';
import {useLayoutEffect,useRef,useState,type CSSProperties,type PointerEvent} from 'react';
import type {Playback} from './types';
import {duration as formatTime} from './components';
import {animateOwned,cancelOwned} from './presentationMotion';
type Drag={pointerId:number;left:number;width:number;maximum:number};
export default function PlaybackProgress({trackId,entryId,position,duration,seek,showTime=true,revealTime=false,transition,live=true}:{revealTime?:boolean;live?:boolean;showTime?:boolean;transition?:Playback['dj_transition'];trackId?:number;entryId?:number;position:number;duration:number;seek:(position:number)=>Promise<void>}){
 const clock=usePlaybackClock(live);if(live&&clock.trackId===trackId)position=clock.position;
 const [draft,setDraft]=useState<number|null>(null),[pending,setPending]=useState(false),[scrubbing,setScrubbing]=useState(false),[error,setError]=useState('');
 const input=useRef<HTMLInputElement>(null),host=useRef<HTMLDivElement>(null),slot=useRef<HTMLDivElement>(null),rail=useRef<HTMLDivElement>(null),time=useRef<HTMLOutputElement>(null),drag=useRef<Drag|null>(null),operation=useRef(0);
 const identity=`${trackId??''}:${entryId??''}`,last=useRef(identity);
 const maximum=Math.max(0,Number.isFinite(duration)?duration:0),value=Math.max(0,Math.min(maximum,draft??position)),fraction=maximum>0?value/maximum:0;
 function release(){const pointer=drag.current;drag.current=null;setScrubbing(false);slot.current?.style.removeProperty('--frozen-rail-width');if(pointer&&input.current?.hasPointerCapture?.(pointer.pointerId))input.current.releasePointerCapture(pointer.pointerId);}
 function cancel(){release();setDraft(null);}
 useLayoutEffect(()=>{
  operation.current++;cancel();setPending(false);setError('');
  const previous=last.current;last.current=identity;
  if(previous!==identity&&previous.split(':')[0]&&trackId&&rail.current)animateOwned(rail.current,'song-progress-handoff',[{clipPath:'inset(0 100% 0 0)',opacity:.4},{clipPath:'inset(0 0% 0 0)',opacity:1}],180);
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
  <div ref={slot} className="playback-rail-slot"><div ref={rail} className="progress-rail" aria-hidden="true"><span style={{width:'100%',transform:`scaleX(${fraction})`}}/></div>
   <input ref={input} className="seek" type="range" aria-label="Playback position" aria-valuetext={`${formatTime(value)} of ${formatTime(maximum)}`} min={0} max={Math.max(1,maximum)} step={.1} value={value} disabled={!trackId||maximum===0} style={{'--progress':`${fraction*100}%`} as CSSProperties}
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
   {transition&&(transition.status==='scheduled'||transition.active)&&transition.seconds>0&&maximum>0&&<span className="transition-region" style={{left:`${Math.min(100,transition.outgoing_cue/maximum*100)}%`,width:`${Math.max(.25,Math.min(100-transition.outgoing_cue/maximum*100,(transition.source_seconds??transition.seconds)/maximum*100))}%`}}><button aria-label={`DJ transition ${transition.active?'active':'scheduled'} at ${formatTime(transition.outgoing_cue)}, ${transition.seconds.toFixed(1)} seconds`} title={`DJmode ${transition.active?'mixing':'starts'} · ${formatTime(transition.outgoing_cue)} · ${transition.seconds.toFixed(1)} s`}/></span>}
  </div>
  {showTime&&trackId&&<output ref={time} className="playback-time" aria-label={revealTime?`${formatTime(maximum-value)} remaining`:undefined}>{revealTime?<span className="remaining-time">−{formatTime(maximum-value)}</span>:<>{formatTime(value)} <span>/ {formatTime(maximum)}</span></>}</output>}
  {error&&<span className="seek-error" role="alert">{error}</span>}
 </div>;
}
