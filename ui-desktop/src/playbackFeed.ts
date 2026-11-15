import {sourcePosition,sourceCorrection} from './playbackClock';
import {useCallback,useRef,useState,useSyncExternalStore} from 'react';
import {useViewActive} from './ViewScope';
import {mergePlaybackUpdate} from './playbackPresentation';
import type {Playback} from './types';
export type Clock={receivedAt:number;playing:boolean;speed:number;trackId:number|undefined;position:number;correction?:number;levels:number[];reverbFrames:number};
const listeners=new Set<()=>void>();
let clock:Clock={receivedAt:0,playing:false,speed:1,trackId:undefined,position:0,levels:[],reverbFrames:0};
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};};
const quiet=()=>()=>{};
export function usePlaybackClock(enabled=true){const active=useViewActive();return useSyncExternalStore(enabled&&active?subscribe:quiet,()=>clock);}
const clockKeys=new Set<keyof Playback>(['position','audio_levels','reverb_processed_frames']);

function equalPresentationValue(a:unknown,b:unknown):boolean{
 if(a===b)return true;
 if(a===null||b===null||typeof a!=='object'||typeof b!=='object')return false;
 if(Array.isArray(a)){
  return Array.isArray(b)&&a.length===b.length&&a.every((value,index)=>equalPresentationValue(value,b[index]));
 }
 if(Array.isArray(b))return false;
 const left=a as Record<string,unknown>,right=b as Record<string,unknown>,keys=Object.keys(left);
 return keys.length===Object.keys(right).length&&keys.every(key=>
  Object.hasOwn(right,key)&&equalPresentationValue(left[key],right[key]));
}

export function samePlaybackPresentation(a:Playback,b:Playback,updated:Partial<Playback>=b){
 return (Object.keys(updated) as (keyof Playback)[]).every(key=>clockKeys.has(key)||equalPresentationValue(a[key],b[key]));
}
/** Publish fast clock/meter ticks only to their visible consumers. Pages receive state changes. */
export function usePlaybackFeed(initial:Playback){
 const latest=useRef(initial),shown=useRef(initial),[presentation,setPresentation]=useState(initial);
 const update=useCallback((next:Partial<Playback>|((old:Playback)=>Playback))=>{
  const patch=typeof next==='function'?next(latest.current):next;
  const value=mergePlaybackUpdate(latest.current,patch);latest.current=value;
  const incoming=value.dj_transition?.active&&value.queue_index!==null&&value.queue_entry_ids?.[value.queue_index]===value.dj_transition.incoming_entry;
  const speed=value.listening_applied===false?clock.speed:(value.listening?.speed??1)*(incoming?value.dj_transition!.tempo:1),sameAnchor=clock.trackId===value.track?.id&&clock.playing===value.is_playing&&clock.speed===speed&&clock.position===value.position;
  const at=performance.now(),anchor={receivedAt:at,playing:value.is_playing,speed,trackId:value.track?.id,position:value.position};
  const correction=sameAnchor?clock.correction:sourceCorrection(clock,anchor,at);
  clock={correction,receivedAt:sameAnchor?clock.receivedAt:at,playing:value.is_playing,speed,trackId:value.track?.id,position:value.position,levels:value.audio_levels??[],reverbFrames:value.reverb_processed_frames??0};listeners.forEach(listener=>listener());
  if(!samePlaybackPresentation(shown.current,value,patch)){shown.current=value;setPresentation(value);}
 },[]);
 return [presentation,update] as const;
}

export function currentPlaybackClock(){return clock;}
export function clockPosition(value:Clock,at=performance.now()){return sourcePosition(value,at);}

export function subscribePlaybackClock(listener:()=>void){return subscribe(listener);}
