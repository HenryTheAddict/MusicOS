import {useCallback,useRef,useState,useSyncExternalStore} from 'react';
import {useViewActive} from './ViewScope';
import {mergePlaybackUpdate} from './playbackPresentation';
import type {Playback} from './types';
type Clock={trackId:number|undefined;position:number;levels:number[];reverbFrames:number};
const listeners=new Set<()=>void>();
let clock:Clock={trackId:undefined,position:0,levels:[],reverbFrames:0};
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};};
const quiet=()=>()=>{};
export function usePlaybackClock(enabled=true){const active=useViewActive();return useSyncExternalStore(enabled&&active?subscribe:quiet,()=>clock);}
export function samePlaybackPresentation(a:Playback,b:Playback){
 return (Object.keys(b) as (keyof Playback)[]).every(key=>['position','audio_levels','reverb_processed_frames'].includes(key)||a[key]===b[key]||(typeof b[key]==='object'&&JSON.stringify(a[key])===JSON.stringify(b[key])));
}
/** Publish fast clock/meter ticks only to their visible consumers. Pages receive state changes. */
export function usePlaybackFeed(initial:Playback){
 const latest=useRef(initial),shown=useRef(initial),[presentation,setPresentation]=useState(initial);
 const update=useCallback((next:Playback|((old:Playback)=>Playback))=>{
  const value=mergePlaybackUpdate(latest.current,typeof next==='function'?next(latest.current):next);latest.current=value;
  clock={trackId:value.track?.id,position:value.position,levels:value.audio_levels??[],reverbFrames:value.reverb_processed_frames??0};listeners.forEach(listener=>listener());
  if(!samePlaybackPresentation(shown.current,value)){shown.current=value;setPresentation(value);}
 },[]);
 return [presentation,update] as const;
}
