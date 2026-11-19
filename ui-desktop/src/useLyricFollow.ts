import {useLayoutEffect,useRef,type RefObject} from 'react';
import type {LyricDocument} from './lyrics';

/** Keep cues mounted. Only the scroll offset moves; no text scales or reflows. */
export function useLyricFollow(host:RefObject<HTMLDivElement|null>,{
 active,following,doc,trackId,variant,focus,last,layoutEpoch,onPosition,
}:{active:boolean;following:boolean;doc:LyricDocument|null|undefined;trackId:number|undefined;variant:string;focus:number|string;last:number|string;layoutEpoch:number;onPosition?:()=>void}){
 const sync=useRef(onPosition);sync.current=onPosition;
 const previous=useRef({doc,trackId,variant,initialized:false});
 useLayoutEffect(()=>{
  const node=host.current,first=node?.querySelector<HTMLElement>(`[data-line="${focus}"]`);
  if(!active||!following||!node||!first||document.hidden)return;
  const media=matchMedia('(prefers-reduced-motion: reduce)'),from=node.scrollTop;
  const box=node.getBoundingClientRect(),a=first.getBoundingClientRect(),b=node.querySelector<HTMLElement>(`[data-line="${last}"]`)?.getBoundingClientRect()??a;
  const to=Math.max(0,Math.min(node.scrollHeight-node.clientHeight,from+a.top-box.top-node.clientHeight*.46+Math.min(node.clientHeight*.7,b.bottom-a.top)/2));
  const firstLayout=!previous.current.initialized||previous.current.doc!==doc||previous.current.trackId!==trackId||previous.current.variant!==variant;
  previous.current={doc,trackId,variant,initialized:true};
  if(firstLayout||media.matches||Math.abs(to-from)<.5){node.scrollTop=to;sync.current?.();return;}
  let frame=0;const began=performance.now(),duration=variant==='full'?180:240;
  const stop=()=>{cancelAnimationFrame(frame);frame=0;};
  const reduced=()=>{if(media.matches){stop();node.scrollTop=to;sync.current?.();}};
  const visibility=()=>{if(document.hidden)stop();};
  const draw=(at:number)=>{frame=0;if(document.hidden)return;const t=Math.min(1,Math.max(0,(at-began)/duration));node.scrollTop=from+(to-from)*(1-Math.pow(1-t,3));sync.current?.();if(t<1)frame=requestAnimationFrame(draw);};
  frame=requestAnimationFrame(draw);media.addEventListener('change',reduced);document.addEventListener('visibilitychange',visibility);
  return()=>{stop();media.removeEventListener('change',reduced);document.removeEventListener('visibilitychange',visibility);};
 },[active,following,doc,trackId,variant,focus,last,layoutEpoch,host]);
}
