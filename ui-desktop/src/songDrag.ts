import type {PointerEvent as ReactPointerEvent} from 'react';
import type {Track} from './types';
export type SongDrag={id:number;ids?:number[];x:number;y:number;drop:boolean;cancelled:boolean};
let cancelCurrent:(()=>void)|undefined,feedback:((label:string,allowed:boolean)=>void)|undefined;
export function songDragFeedback(label:string,allowed=true){feedback?.(label,allowed);}
// A press remains a click until it crosses the drag threshold. Capture only then.
export function startSongDrag(event:ReactPointerEvent,track:Track,ids:number[]=[track.id]){
 if(event.button!==0||event.pointerType==='touch')return;
 cancelCurrent?.();
 const target=event.currentTarget as HTMLElement,pointer=event.pointerId,start={x:event.clientX,y:event.clientY},ordered=[...new Set(ids)];
 let x=start.x,y=start.y,moving=false,done=false,ghost:HTMLDivElement|null=null,hint:HTMLElement|null=null,frame=0;
 const emit=(drop=false,cancelled=false)=>window.dispatchEvent(new CustomEvent<SongDrag>('musicos-song-drag',{detail:{id:track.id,ids:ordered,x,y,drop,cancelled}}));
 const suppressClick=()=>{const block=(e:MouseEvent)=>{e.preventDefault();e.stopImmediatePropagation();};window.addEventListener('click',block,{once:true,capture:true});setTimeout(()=>window.removeEventListener('click',block,true),0);};
 const scrollTarget=()=>document.elementFromPoint(x,y)?.closest<HTMLElement>('.sidebar-playlist-list,.sidebar-body,.page-scroll');
 const velocity=(node:HTMLElement)=>{const box=node.getBoundingClientRect(),edge=Math.min(36,box.height/4);return y<box.top+edge?-Math.ceil((box.top+edge-y)/edge*8):y>box.bottom-edge?Math.ceil((y-box.bottom+edge)/edge*8):0;};
 const autoscroll=()=>{frame=0;if(!moving||done)return;const node=scrollTarget();if(!node)return;const speed=velocity(node),before=node.scrollTop;if(speed)node.scrollTop+=speed;if(node.scrollTop!==before){emit();frame=requestAnimationFrame(autoscroll);}};
 const move=(e:PointerEvent)=>{
  if(e.pointerId!==pointer||done)return;x=e.clientX;y=e.clientY;
  if(!target.isConnected){cancel();return;}
  if(!moving&&Math.hypot(x-start.x,y-start.y)<6)return;
  if(!moving){moving=true;target.setPointerCapture?.(pointer);target.dataset.songDragging='true';ghost=document.createElement('div');ghost.className='song-drag-preview';ghost.setAttribute('aria-hidden','true');const title=document.createElement('strong');title.textContent=ordered.length>1?`${ordered.length} songs`:track.title;hint=document.createElement('small');ghost.append(title,hint);document.querySelector('.app-shell')?.appendChild(ghost);feedback=(label,allowed)=>{if(hint)hint.textContent=label;if(ghost)ghost.dataset.allowed=String(allowed);};}
  e.preventDefault();window.getSelection()?.removeAllRanges();
  if(ghost){ghost.style.left=`${Math.max(8,Math.min(x+12,innerWidth-ghost.offsetWidth-8))}px`;ghost.style.top=`${Math.max(8,Math.min(y+12,innerHeight-ghost.offsetHeight-8))}px`;}
  songDragFeedback('Drop onto a playlist or Up next');emit();if(!frame)frame=requestAnimationFrame(autoscroll);
 };
 const cleanup=()=>{if(done)return;done=true;cancelAnimationFrame(frame);window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',up);window.removeEventListener('pointercancel',cancelPointer);window.removeEventListener('keydown',escape,true);window.removeEventListener('dragstart',native,true);window.removeEventListener('contextmenu',cancel,true);window.removeEventListener('blur',cancel);window.removeEventListener('musicos-navigation-start',cancel);document.removeEventListener('visibilitychange',visibility);delete target.dataset.songDragging;ghost?.remove();if(target.hasPointerCapture?.(pointer))target.releasePointerCapture(pointer);cancelCurrent=undefined;feedback=undefined;};
 const up=(e:PointerEvent)=>{if(e.pointerId!==pointer)return;x=e.clientX;y=e.clientY;if(moving){emit(true,!target.isConnected);suppressClick();}cleanup();};
 const cancel=()=>{if(moving)emit(true,true);cleanup();};
 const cancelPointer=(e:PointerEvent)=>{if(e.pointerId===pointer)cancel();};
 const visibility=()=>{if(document.hidden)cancel();};
 const escape=(e:KeyboardEvent)=>{if(e.key!=='Escape')return;if(moving){e.preventDefault();e.stopImmediatePropagation();const release=(event:PointerEvent)=>{if(event.pointerId!==pointer)return;remove();if(event.type==='pointerup')suppressClick();};const remove=()=>{window.removeEventListener('pointerup',release);window.removeEventListener('pointercancel',release);window.removeEventListener('blur',remove);};window.addEventListener('pointerup',release);window.addEventListener('pointercancel',release);window.addEventListener('blur',remove);}cancel();};
 const native=(e:DragEvent)=>e.preventDefault();
 cancelCurrent=cancel;
 window.addEventListener('pointermove',move,{passive:false});window.addEventListener('pointerup',up);window.addEventListener('pointercancel',cancelPointer);window.addEventListener('keydown',escape,true);window.addEventListener('dragstart',native,true);window.addEventListener('contextmenu',cancel,true);window.addEventListener('blur',cancel);window.addEventListener('musicos-navigation-start',cancel);document.addEventListener('visibilitychange',visibility);
}
