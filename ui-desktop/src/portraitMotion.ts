import {useLayoutEffect,type RefObject} from 'react';
import {springStep} from './motion';
export function useArtistPointer(root:RefObject<HTMLElement|null>,enabled=true){
 useLayoutEffect(()=>{const stage=root.current;if(!stage||!enabled)return;const button=stage.querySelector<HTMLElement>('.artist-portrait-button,.album-artwork');if(!button)return;
 const reduced=matchMedia('(prefers-reduced-motion: reduce)');let frame=0,last=0,x=0,y=0,vx=0,vy=0,tx=0,ty=0,inside=false,bounds:DOMRect|null=null;
 const animate=(now:number)=>{frame=0;const dt=last?Math.min(1,(now-last)/1000):1/120;last=now;
  [x,vx]=springStep(x,vx,tx,dt);[y,vy]=springStep(y,vy,ty,dt);
  button.style.setProperty('--portrait-x',`${x.toFixed(3)}px`);button.style.setProperty('--portrait-y',`${y.toFixed(3)}px`);button.style.setProperty('--portrait-scale',inside?'1.025':'1');
  if(Math.abs(x-tx)+Math.abs(y-ty)+Math.abs(vx)+Math.abs(vy)>.02)frame=requestAnimationFrame(animate);else last=0;
 };
 const start=()=>{if(!frame&&!reduced.matches)frame=requestAnimationFrame(animate);};
 const enter=()=>{inside=true;bounds=button.getBoundingClientRect();};
 const move=(e:PointerEvent)=>{if(!inside||!bounds||reduced.matches)return;tx=Math.max(-2.5,Math.min(2.5,(e.clientX-bounds.left-bounds.width/2)/bounds.width*5));ty=Math.max(-2.5,Math.min(2.5,(e.clientY-bounds.top-bounds.height/2)/bounds.height*5));start();};
 const leave=()=>{inside=false;tx=ty=0;start();};const resize=()=>{if(inside)bounds=button.getBoundingClientRect();};
 const reset=()=>{cancelAnimationFrame(frame);frame=0;last=0;tx=ty=x=y=vx=vy=0;for(const name of ['--portrait-x','--portrait-y','--portrait-scale'])button.style.removeProperty(name);};
 button.addEventListener('pointerenter',enter);button.addEventListener('pointermove',move);button.addEventListener('pointerleave',leave);window.addEventListener('resize',resize);reduced.addEventListener('change',reset);
 return()=>{reset();button.removeEventListener('pointerenter',enter);button.removeEventListener('pointermove',move);button.removeEventListener('pointerleave',leave);window.removeEventListener('resize',resize);reduced.removeEventListener('change',reset);};
 },[root,enabled]);
}
