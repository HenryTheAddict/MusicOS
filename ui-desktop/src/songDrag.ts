import type {PointerEvent as ReactPointerEvent} from 'react';
import type {Track} from './types';
import {springStep} from './motion';
export type SongDrag={id:number;ids?:number[];x:number;y:number;drop:boolean;cancelled:boolean};
let cancelCurrent:(()=>void)|undefined,feedback:((label:string,allowed:boolean)=>void)|undefined,retirePreview:(()=>void)|undefined;
export function songDragFeedback(label:string,allowed=true){feedback?.(label,allowed);}
const clamp=(n:number,min:number,max:number)=>Math.max(min,Math.min(max,n));
// A press remains a click until it crosses the drag threshold. Capture only then.
export function startSongDrag(event:ReactPointerEvent,track:Track,ids:number[]=[track.id]){
 if(event.button!==0||event.pointerType==='touch')return;
 cancelCurrent?.();retirePreview?.();
 const target=event.currentTarget as HTMLElement,pointer=event.pointerId,start={x:event.clientX,y:event.clientY},ordered=[...new Set(ids)];
 const reduced=window.matchMedia('(prefers-reduced-motion: reduce)');
 let x=start.x,y=start.y,moving=false,done=false,ghost:HTMLDivElement|null=null,inner:HTMLDivElement|null=null,hint:HTMLElement|null=null,frame=0,observer:ResizeObserver|undefined;
 let width=0,height=0,last=performance.now(),moved=last,vx=0,vy=0,sx=0,sy=0,tilt=0,svx=0,svy=0,tv=0;
 const frozen=new Map<HTMLElement,{transform:string;opacity:string}>();
 const emit=(drop=false,cancelled=false)=>window.dispatchEvent(new CustomEvent<SongDrag>('musicos-song-drag',{detail:{id:track.id,ids:ordered,x,y,drop,cancelled}}));
 const suppressClick=()=>{const block=(e:MouseEvent)=>{e.preventDefault();e.stopImmediatePropagation();};window.addEventListener('click',block,{once:true,capture:true});setTimeout(()=>window.removeEventListener('click',block,true),0);};
 const scrollTarget=()=>document.elementFromPoint(x,y)?.closest<HTMLElement>('.sidebar-playlist-list,.sidebar-body,.page-scroll');
 const velocity=(node:HTMLElement)=>{const box=node.getBoundingClientRect(),edge=Math.min(36,box.height/4);return y<box.top+edge?-Math.ceil((box.top+edge-y)/edge*8):y>box.bottom-edge?Math.ceil((y-box.bottom+edge)/edge*8):0;};
 const schedule=()=>{if(!frame&&!done)frame=requestAnimationFrame(draw);};
 const draw=(now:number)=>{
  frame=0;if(!moving||done||!ghost||!inner)return;if(!target.isConnected){cancel();return;}
  const dt=clamp((now-last)/1000,0,.032);last=now;const recent=now-moved<70,speed=recent?Math.hypot(vx,vy):0;
  const stretch=clamp(speed/50000,0,.025),horizontal=Math.abs(vx)>=Math.abs(vy),tx=recent?(horizontal?stretch:-stretch*.65):0,ty=recent?(horizontal?-stretch*.65:stretch):0,tr=recent?clamp(vx/650,-2.5,2.5):0;
  if(reduced.matches||(!recent&&now-moved>=600)){sx=sy=tilt=svx=svy=tv=0;}else{[sx,svx]=springStep(sx,svx,tx,dt);[sy,svy]=springStep(sy,svy,ty,dt);[tilt,tv]=springStep(tilt,tv,tr,dt);}
  ghost.style.transform=`translate3d(${clamp(x+12,8,Math.max(8,innerWidth-width-8))}px,${clamp(y+12,8,Math.max(8,innerHeight-height-8))}px,0)`;
  inner.style.transform=`rotate(${clamp(tilt,-3,3)}deg) scale(${1+clamp(sx,-.03,.03)},${1+clamp(sy,-.03,.03)})`;
  const node=scrollTarget();let scrolled=false;if(node){const speed=velocity(node),before=node.scrollTop;if(speed)node.scrollTop+=speed;scrolled=node.scrollTop!==before;if(scrolled)emit();}
  if(scrolled||(!reduced.matches&&(recent||Math.abs(sx)+Math.abs(sy)+Math.abs(svx)+Math.abs(svy)+Math.abs(tilt)+Math.abs(tv)>.001)))schedule();
 };
 const move=(e:PointerEvent)=>{
  if(e.pointerId!==pointer||done)return;const now=performance.now(),dt=Math.max(8,now-moved);vx=clamp((e.clientX-x)*1000/dt,-2500,2500);vy=clamp((e.clientY-y)*1000/dt,-2500,2500);x=e.clientX;y=e.clientY;moved=now;
  if(!target.isConnected){cancel();return;}if(!moving&&Math.hypot(x-start.x,y-start.y)<6)return;
  if(!moving){
   moving=true;
   target.querySelectorAll<HTMLElement>('.track-label,.row-play').forEach(el=>{const style=getComputedStyle(el);frozen.set(el,{transform:el.style.transform,opacity:el.style.opacity});el.style.transform=style.transform;el.style.opacity=style.opacity;});
   target.setPointerCapture?.(pointer);target.dataset.songDragging='true';
   ghost=document.createElement('div');ghost.className='song-drag-preview';ghost.setAttribute('aria-hidden','true');inner=document.createElement('div');inner.className='song-drag-preview-inner';
   const title=document.createElement('strong');title.textContent=ordered.length>1?`${ordered.length} songs`:track.title;hint=document.createElement('small');inner.append(title,hint);ghost.append(inner);document.querySelector('.app-shell')?.appendChild(ghost);
   feedback=(label,allowed)=>{if(hint&&hint.textContent!==label)hint.textContent=label;if(ghost)ghost.dataset.allowed=String(allowed);};
   observer=new ResizeObserver(entries=>{width=entries[0].borderBoxSize?.[0]?.inlineSize??ghost?.offsetWidth??0;height=entries[0].borderBoxSize?.[0]?.blockSize??ghost?.offsetHeight??0;schedule();});observer.observe(ghost);last=now;
  }
  e.preventDefault();window.getSelection()?.removeAllRanges();songDragFeedback('Drop onto a playlist or Up next');emit();schedule();
 };
 const cleanup=(settle=false)=>{
  if(done)return;done=true;cancelAnimationFrame(frame);observer?.disconnect();
  window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',up);window.removeEventListener('pointercancel',cancelPointer);window.removeEventListener('keydown',escape,true);window.removeEventListener('dragstart',native,true);window.removeEventListener('contextmenu',cancel,true);window.removeEventListener('blur',cancel);window.removeEventListener('musicos-navigation-start',cancel);document.removeEventListener('visibilitychange',visibility);
  frozen.forEach((style,el)=>{el.style.transform=style.transform;el.style.opacity=style.opacity;});delete target.dataset.songDragging;target.dispatchEvent(new Event('musicos-song-drag-end'));
  if(target.hasPointerCapture?.(pointer))target.releasePointerCapture(pointer);cancelCurrent=undefined;feedback=undefined;
  const preview=ghost,layer=inner;
  if(settle&&preview&&layer&&!reduced.matches){
   preview.dataset.settling='true';const animation=layer.animate([{transform:layer.style.transform,opacity:1},{transform:'rotate(0deg) scale(.98)',opacity:0}],{duration:180,easing:'ease-out'});
   const remove=()=>{animation.cancel();preview.remove();window.removeEventListener('musicos-navigation-start',remove);window.removeEventListener('blur',remove);if(retirePreview===remove)retirePreview=undefined;};
   retirePreview=remove;animation.onfinish=remove;window.addEventListener('musicos-navigation-start',remove,{once:true});window.addEventListener('blur',remove,{once:true});
  }else preview?.remove();
 };
 const up=(e:PointerEvent)=>{if(e.pointerId!==pointer)return;x=e.clientX;y=e.clientY;if(moving){emit(true,!target.isConnected);suppressClick();}cleanup(moving&&target.isConnected);};
 const cancel=()=>{if(moving)emit(true,true);cleanup();};
 const cancelPointer=(e:PointerEvent)=>{if(e.pointerId===pointer)cancel();};
 const visibility=()=>{if(document.hidden)cancel();};
 const escape=(e:KeyboardEvent)=>{if(e.key!=='Escape')return;if(moving){e.preventDefault();e.stopImmediatePropagation();const release=(event:PointerEvent)=>{if(event.pointerId!==pointer)return;remove();if(event.type==='pointerup')suppressClick();};const remove=()=>{window.removeEventListener('pointerup',release);window.removeEventListener('pointercancel',release);window.removeEventListener('blur',remove);};window.addEventListener('pointerup',release);window.addEventListener('pointercancel',release);window.addEventListener('blur',remove);}cancel();};
 const native=(e:DragEvent)=>e.preventDefault();cancelCurrent=cancel;
 window.addEventListener('pointermove',move,{passive:false});window.addEventListener('pointerup',up);window.addEventListener('pointercancel',cancelPointer);window.addEventListener('keydown',escape,true);window.addEventListener('dragstart',native,true);window.addEventListener('contextmenu',cancel,true);window.addEventListener('blur',cancel);window.addEventListener('musicos-navigation-start',cancel);document.addEventListener('visibilitychange',visibility);
}
