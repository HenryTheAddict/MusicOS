import {useEffect,useRef,type RefObject} from 'react';
export const backSwipeThreshold=110;
export function acceptsBackAxis(x:number,y:number){return x< -10&&Math.abs(x)>Math.abs(y)*1.8;}
export function consumesHorizontalScroll(target:Element|null,root:Element){
 for(let node=target;node&&node!==root;node=node.parentElement){
  if(node.matches('input,textarea,select,[contenteditable=true],[role=slider],dialog,[role=menu],.queue-scroll,.mini-queue-list'))return true;
  const style=getComputedStyle(node);if(/auto|scroll/.test(style.overflowX)&&node.scrollWidth>node.clientWidth+2)return true;
 }
 return false;
}
/** A finite, axis-locked trackpad gesture. No wheel handler is installed in Mini Player. */
export function useBackSwipe(root:RefObject<HTMLElement|null>,enabled:boolean,canBack:boolean,onBack:()=>void){
 const latest=useRef({enabled,canBack,onBack});latest.current={enabled,canBack,onBack};
 const cue=useRef<HTMLDivElement>(null);
 useEffect(()=>{
  const host=root.current;if(!host||!enabled)return;
  let x=0,y=0,axis:'pending'|'horizontal'|'ignored'='pending',last=0,locked=false,timer=0,frame=0,amount=0;
  const paint=()=>{frame=0;const node=cue.current;if(!node)return;const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;node.style.opacity=amount>0?'1':'0';node.style.transform=reduced?'none':`translateX(${Math.round(-34+Math.min(1,amount)*42)}px)`;node.dataset.ready=String(amount>=1);};
  const draw=(next:number)=>{amount=next;if(!frame)frame=requestAnimationFrame(paint);};
  const reset=()=>{x=0;y=0;axis='pending';locked=false;draw(0);};
  const settle=()=>{timer=0;const complete=axis==='horizontal'&&-x>=backSwipeThreshold;draw(0);if(complete&&!locked&&latest.current.enabled&&latest.current.canBack&&!document.querySelector('dialog[open],[role=menu],.audio-panel')){locked=true;latest.current.onBack();}x=0;y=0;axis='pending';};
  const wheel=(event:WheelEvent)=>{
   if(event.defaultPrevented||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||event.deltaMode!==0||!latest.current.enabled||!latest.current.canBack||document.querySelector('dialog[open],[role=menu],.audio-panel'))return;
   const now=performance.now();if(now-last>260)reset();last=now;clearTimeout(timer);timer=window.setTimeout(settle,130);
   if(locked||axis==='ignored')return;
   if(consumesHorizontalScroll(event.target as Element,host)){axis='ignored';return;}
   x+=event.deltaX;y+=event.deltaY;
   if(axis==='pending'){
    if(Math.abs(x)<12&&Math.abs(y)<12)return;
    if(!acceptsBackAxis(x,y)){axis='ignored';return;}axis='horizontal';
   }
   if(Math.abs(y)>Math.abs(x)*.8||x>0){axis='ignored';draw(0);return;}
   event.preventDefault();draw(Math.min(1.15,Math.max(0,-x/backSwipeThreshold)));
  };
  const cancel=()=>{clearTimeout(timer);locked=true;draw(0);};
  host.addEventListener('wheel',wheel,{passive:false});window.addEventListener('resize',cancel);window.addEventListener('blur',cancel);window.addEventListener('pointerdown',cancel);window.addEventListener('musicos-navigation-start',cancel);
  return()=>{host.removeEventListener('wheel',wheel);window.removeEventListener('resize',cancel);window.removeEventListener('blur',cancel);window.removeEventListener('pointerdown',cancel);window.removeEventListener('musicos-navigation-start',cancel);clearTimeout(timer);cancelAnimationFrame(frame);if(cue.current){cue.current.style.opacity='0';cue.current.style.transform='none';}};
 },[root,enabled]);
 return cue;
}
