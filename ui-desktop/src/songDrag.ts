import type {PointerEvent as ReactPointerEvent} from 'react';
import type {Track} from './types';

export type SongDrag = {id:number;ids?:number[];x:number;y:number;drop:boolean;cancelled:boolean};
// Pointer drags stay inside MusicOS and never become operating-system file drops.
export function startSongDrag(event:ReactPointerEvent,track:Track,ids:number[]=[track.id]) {
  if(event.button!==0||event.pointerType==='touch')return;
  event.preventDefault();
  const target=event.currentTarget as HTMLElement;
  target.setPointerCapture?.(event.pointerId);
  const start={x:event.clientX,y:event.clientY},pointer=event.pointerId;
  let moving=false,ghost:HTMLDivElement|null=null;
  const emit=(x:number,y:number,drop=false,cancelled=false)=>window.dispatchEvent(new CustomEvent<SongDrag>('musicos-song-drag',{detail:{id:track.id,ids,x,y,drop,cancelled}}));
  const move=(e:PointerEvent)=>{
    if(e.pointerId!==pointer)return;
    if(!moving&&Math.hypot(e.clientX-start.x,e.clientY-start.y)<6)return;
    if(!moving){moving=true;ghost=document.createElement('div');ghost.className='song-drag-preview';ghost.textContent=track.title;document.querySelector('.app-shell')?.appendChild(ghost);}
    e.preventDefault();window.getSelection()?.removeAllRanges();
    if(ghost){ghost.style.left=`${e.clientX+12}px`;ghost.style.top=`${e.clientY+12}px`;}
    emit(e.clientX,e.clientY);
  };
  const finish=(e:PointerEvent,cancelled=false)=>{
    if(e.pointerId!==pointer)return;
    if(moving){emit(e.clientX,e.clientY,true,cancelled);suppressClick();}
    cleanup();
  };
  const suppress=(e:MouseEvent)=>{e.preventDefault();e.stopImmediatePropagation();};
  const suppressClick=()=>{window.addEventListener('click',suppress,{once:true,capture:true});setTimeout(()=>window.removeEventListener('click',suppress,true),0);};
  const up=(e:PointerEvent)=>finish(e);
  const cancel=(e:PointerEvent)=>finish(e,true);
  const escape=(e:KeyboardEvent)=>{if(e.key==='Escape'){if(moving){e.preventDefault();e.stopImmediatePropagation();emit(0,0,true,true);
    // Releasing capture after Escape can still synthesize a click on the
    // source. Consume that click only when this pointer is actually released.
    const released=(event:PointerEvent)=>{if(event.pointerId!==pointer)return;window.removeEventListener('pointerup',released);window.removeEventListener('pointercancel',released);if(event.type==='pointerup')suppressClick();};
    window.addEventListener('pointerup',released);window.addEventListener('pointercancel',released);
  }cleanup();}};
  const native=(e:DragEvent)=>{e.preventDefault();};
  const cleanup=()=>{if(target.hasPointerCapture?.(pointer))target.releasePointerCapture(pointer);ghost?.remove();window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',up);window.removeEventListener('pointercancel',cancel);window.removeEventListener('keydown',escape,true);window.removeEventListener('dragstart',native,true);};
  window.addEventListener('pointermove',move,{passive:false});window.addEventListener('pointerup',up);window.addEventListener('pointercancel',cancel);window.addEventListener('keydown',escape,true);window.addEventListener('dragstart',native,true);
}
