import {useEffect,useRef,useState,type ReactNode} from 'react';
import {request} from './bridge';
import {displayedPlayerOffset,playerEntrance} from './playerMotion';
export function feedback(kind='selection'){void request({action:'haptic',kind}).catch(()=>{});}
/** One native touch owner locks only its intended axis; vertical lists keep native scrolling. */
export function useSwipe(onSwipe:(direction:'left'|'right'|'up'|'down',pulledOffset?:number)=>void,axis:'horizontal'|'vertical'|'both'='horizontal',mode:'all'|'edge'|'off'='all',pullSheet=false){
 const [node,ref]=useState<HTMLElement|null>(null);
 const options=useRef({onSwipe,axis,mode,pullSheet});options.current={onSwipe,axis,mode,pullSheet};
 const suppress=useRef(0);
 useEffect(()=>{
  if(!node)return;
  type Axis = 'horizontal' | 'vertical';
  let start:{x:number;y:number;id:number;axis?:Axis}|null=null;
  let frame=0,x=0,y=0,motion=true,sheet:HTMLElement|null=null;
  let initialSheetOffset=0;
  const clear=()=>{
   cancelAnimationFrame(frame);frame=0;start=null;
   node.removeAttribute('data-swiping');node.style.removeProperty('--swipe-x');node.style.removeProperty('--swipe-y');
   sheet?.removeAttribute('data-pulling');
   sheet?.style.removeProperty('--player-pull');
   sheet=null;
   initialSheetOffset=0;
  };
  const draw=()=>{
   frame=0;
   if(!motion)return;
   node.style.setProperty('--swipe-x',`${x}px`);
   node.style.setProperty('--swipe-y',`${y}px`);
   if(sheet){
    sheet.setAttribute('data-pulling','true');
    sheet.style.setProperty('--player-pull',`${Math.max(0,y)}px`);
   }
  };
  const begin=(e:TouchEvent)=>{
   if(node.closest('[data-swipe-closing]'))return;
   suppress.current=0;
   clear();
   const {mode}=options.current,target=e.target instanceof Element?e.target:null;
   if(e.touches.length!==1)return;
   const t=e.touches[0];
   if(mode==='off'||!target||target.closest('.icon-button,input,textarea,select,[data-no-swipe],.reorder-handle'))return;
   if(mode==='edge'&&(t.clientX>32||target.closest('.phone-categories')))return;
   if(mode!=='edge'&&t.clientX<=32&&node.closest('[data-edge-back=true]'))return;
   if(mode!=='edge'&&target.closest('[data-swipe-owner]')!==node)return;
   motion=!matchMedia('(prefers-reduced-motion: reduce)').matches&&!node.closest('[data-reduced-motion=true]');
   start={x:t.clientX,y:t.clientY,id:t.identifier};
  };
  const move=(e:TouchEvent)=>{
   if(!start)return;
   if(options.current.mode==='off'||e.touches.length!==1){clear();return;}
   const t=e.touches[0];
   if(t.identifier!==start.id)return;
   const dx=t.clientX-start.x,dy=t.clientY-start.y,{axis,mode,pullSheet}=options.current;
   if(!start.axis){
    if(Math.hypot(dx,dy)<12)return;
    const chosen=Math.abs(dy)>=Math.abs(dx)?'vertical':'horizontal';
    if((axis!=='both'&&axis!==chosen)||(mode==='edge'&&dx<0)||(pullSheet&&chosen==='vertical'&&dy<0)){clear();return;}
    start.axis=chosen;
    node.setAttribute('data-swiping','true');
    if(mode==='edge')node.querySelector('.phone-page')?.getAnimations().forEach(animation=>animation.cancel());
    if(pullSheet&&chosen==='vertical'){
     sheet=node.closest<HTMLElement>('.phone-player');
     if(sheet&&motion){
      initialSheetOffset=displayedPlayerOffset(sheet);
      sheet.getAnimations().find(animation=>animation.id===playerEntrance)?.cancel();
      sheet.setAttribute('data-pulling','true');
      sheet.style.setProperty('--player-pull',`${initialSheetOffset}px`);
     }
    }
   }
   if(e.cancelable)e.preventDefault();
   suppress.current=performance.now()+400;
   const row=node.classList.contains('phone-swipe-song');
   x=start.axis==='vertical'?0:mode==='edge'?Math.max(0,Math.min(node.clientWidth,dx)):row?Math.max(-100,Math.min(100,dx)):Math.max(-60,Math.min(60,dx*.6));
   y=start.axis==='horizontal'?0:sheet?Math.min(Math.max(0,dy+initialSheetOffset),window.innerHeight*.7):Math.max(-60,Math.min(60,dy*.6));
   if(!frame)frame=requestAnimationFrame(draw);
  };
  const end=(e:TouchEvent)=>{
   if(options.current.mode==='off'){clear();return;}
   const s=start,t=e.changedTouches[0];
   if(!s||!t||t.identifier!==s.id)return;
   const distance=s.axis==='horizontal'?t.clientX-s.x:t.clientY-s.y;
   const movingSheet=sheet;
   const displacement=motion&&movingSheet ? Number.parseFloat(movingSheet.style.getPropertyValue('--player-pull'))||0 : 0;
   clear();
   if(!s.axis)return;
   suppress.current=performance.now()+400;
   if(e.cancelable)e.preventDefault();
   if(Math.abs(distance)<54)return;
   const direction=s.axis==='horizontal'?(distance<0?'left':'right'):(distance<0?'up':'down');
   if(options.current.pullSheet&&direction==='up')return;
   feedback();
   options.current.onSwipe(direction,direction==='down'&&movingSheet?displacement:undefined);
  };
  node.addEventListener('touchstart',begin,{passive:true});
  node.addEventListener('touchmove',move,{passive:false});
  node.addEventListener('touchend',end,{passive:false});
  node.addEventListener('touchcancel',clear);window.addEventListener('resize',clear);
  return()=>{clear();node.removeEventListener('touchstart',begin);node.removeEventListener('touchmove',move);node.removeEventListener('touchend',end);node.removeEventListener('touchcancel',clear);window.removeEventListener('resize',clear);};
 },[node]);
 return {ref,'data-swipe-owner':true,'data-edge-back':mode==='edge'||undefined,onClickCapture:(e:React.MouseEvent)=>{if(performance.now()<suppress.current){e.preventDefault();e.stopPropagation();suppress.current=0;}}};
}
export function SwipeSong({children,onNext}:{children:ReactNode;onNext:()=>void}){
 const [open,setOpen]=useState(false),swipe=useSwipe(d=>setOpen(d==='left'));
 return <div className="phone-swipe-song" data-open={open||undefined} {...swipe}><button className="swipe-next" data-no-swipe aria-hidden={!open} tabIndex={open?0:-1} aria-label="Play next" onClick={()=>{onNext();setOpen(false);}}>Play next</button><div className="swipe-song-content">{children}</div></div>;
}
export function ReorderList<T>({items,itemKey,render,onMove,onRemove}:{items:T[];itemKey:(item:T,index:number)=>string|number;render:(item:T,index:number)=>ReactNode;onMove:(from:number,to:number)=>void;onRemove:(index:number)=>void}){
 const host=useRef<HTMLDivElement>(null),drag=useRef<{from:number;to:number;y:number;start:number;node:HTMLElement;pointer:number;bounds:{top:number;bottom:number}[];scroll:HTMLElement;scrollTop:number;edge:DOMRect}|null>(null),frame=useRef(0),[moving,setMoving]=useState<number|null>(null),[target,setTarget]=useState<number|null>(null),oldRects=useRef(new Map<string,DOMRect>()),animations=useRef(new Set<Animation>());
 useEffect(()=>{const cancel=()=>finish(false);window.addEventListener('resize',cancel);return()=>{window.removeEventListener('resize',cancel);cancelAnimationFrame(frame.current);animations.current.forEach(a=>a.cancel());animations.current.clear();drag.current?.node.style.removeProperty('transform');};},[]);
 useEffect(()=>{const nodes=host.current?.querySelectorAll<HTMLElement>('[data-reorder-key]');nodes?.forEach(node=>{const key=node.dataset.reorderKey!,rect=node.getBoundingClientRect(),old=oldRects.current.get(key);if(old&&!drag.current&&!matchMedia('(prefers-reduced-motion:reduce)').matches&&Math.abs(old.top-rect.top)>1){const a=node.animate([{transform:`translateY(${old.top-rect.top}px)`},{transform:'none'}],{duration:180,easing:'cubic-bezier(.2,.75,.2,1)'});animations.current.add(a);void a.finished.then(()=>{a.cancel();animations.current.delete(a);}).catch(()=>animations.current.delete(a));}oldRects.current.set(key,rect);});},[items]);
 function destination(d:NonNullable<typeof drag.current>){const y=d.y+d.scroll.scrollTop-d.scrollTop;const at=d.bounds.findIndex(b=>y<b.bottom);return at<0?d.bounds.length-1:Math.max(0,at);}
 function finish(commit:boolean){const d=drag.current;if(!d)return;if(commit)d.to=destination(d);drag.current=null;cancelAnimationFrame(frame.current);d.node.style.removeProperty('transform');d.node.style.removeProperty('z-index');setMoving(null);setTarget(null);if(commit&&d.from!==d.to){onMove(d.from,d.to);feedback('drop');}}
 function draw(){const d=drag.current;if(!d)return;const edge=d.edge;if(d.y<edge.top+45)d.scroll.scrollTop-=7;else if(d.y>edge.bottom-45)d.scroll.scrollTop+=7;const offset=d.scroll.scrollTop-d.scrollTop;const at=destination(d);if(at!==d.to){d.to=at;setTarget(at);feedback();}d.node.style.transform=`translateY(${d.y-d.start+offset}px)`;frame.current=requestAnimationFrame(draw);}
 return <div className="phone-reorder-list" ref={host}>{items.map((item,index)=><div key={itemKey(item,index)} data-reorder-key={String(itemKey(item,index))} data-dragging={moving===index||undefined} data-drop={moving!==null&&target===index&&target!==moving||undefined} className="phone-reorder-row">{render(item,index)}<button className="icon-button remove-entry" aria-label={`Remove item ${index+1}`} onClick={()=>{onRemove(index);feedback("light");}}><span aria-hidden>−</span></button><button className="icon-button reorder-handle" aria-label={`Reorder item ${index+1}. Use arrow keys to move.`} onKeyDown={e=>{if(e.key==='ArrowUp'&&index>0){e.preventDefault();onMove(index,index-1);feedback();}if(e.key==='ArrowDown'&&index<items.length-1){e.preventDefault();onMove(index,index+1);feedback();}}} onPointerDown={e=>{if(e.button!==0)return;e.preventDefault();const node=e.currentTarget.closest<HTMLElement>('[data-reorder-key]')!,scroll=host.current!.closest<HTMLElement>('[data-reorder-scroll]')??host.current!.parentElement!;drag.current={from:index,to:index,y:e.clientY,start:e.clientY,node,pointer:e.pointerId,bounds:[...host.current!.children].map(n=>n.getBoundingClientRect()),scroll,scrollTop:scroll.scrollTop,edge:scroll.getBoundingClientRect()};e.currentTarget.setPointerCapture(e.pointerId);setMoving(index);setTarget(index);node.style.zIndex='3';feedback('pickup');frame.current=requestAnimationFrame(draw);}} onPointerMove={e=>{if(drag.current?.pointer===e.pointerId)drag.current.y=e.clientY;}} onPointerUp={e=>{if(drag.current?.pointer===e.pointerId){drag.current.y=e.clientY;finish(true);}}} onPointerCancel={()=>finish(false)}><span aria-hidden className="reorder-grip"/></button></div>)}</div>;
}
