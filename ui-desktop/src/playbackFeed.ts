import {useCallback,useRef,useState,useSyncExternalStore} from 'react';
import {useViewActive} from './ViewScope';
import {mergePlaybackUpdate} from './playbackPresentation';
import type {Playback} from './types';
export type Clock={receivedAt:number;playing:boolean;speed:number;trackId:number|undefined;position:number;levels:number[];reverbFrames:number};
const listeners=new Set<()=>void>();
let clock:Clock={receivedAt:0,playing:false,speed:1,trackId:undefined,position:0,levels:[],reverbFrames:0};
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
  const speed=value.listening_applied===false?clock.speed:value.listening?.speed??1,sameAnchor=clock.trackId===value.track?.id&&clock.playing===value.is_playing&&clock.speed===speed&&clock.position===value.position;
  clock={receivedAt:sameAnchor?clock.receivedAt:performance.now(),playing:value.is_playing,speed,trackId:value.track?.id,position:value.position,levels:value.audio_levels??[],reverbFrames:value.reverb_processed_frames??0};listeners.forEach(listener=>listener());
  if(!samePlaybackPresentation(shown.current,value)){shown.current=value;setPresentation(value);}
 },[]);
 return [presentation,update] as const;
}

export function currentPlaybackClock(){return clock;}
export function clockPosition(value:Clock,at=performance.now()){return value.position+(value.playing?Math.min(.5,Math.max(0,(at-value.receivedAt)/1000))*value.speed:0);}

export function subscribePlaybackClock(listener:()=>void){return subscribe(listener);}
