import {animateOwned} from './presentationMotion';
import {useAlbumReveal,coverTransferPending} from "./albumTransitions";
import { useLayoutEffect, useRef, type RefObject } from "react";

// Keep depth geometry synchronized without animating artwork between columns.
export function useGridReflow(root: RefObject<HTMLElement | null>, columns: number, width: number) {
  useLayoutEffect(()=>{root.current?.dispatchEvent(new Event("musicos-grid-layout"));},[root,columns,width]);
}

// Content fades belong to deliberate navigation, never analysis or scrolling.
export function useContentFade(root:RefObject<HTMLElement|null>,location:string,ready=true,selector?:string,id='content-change',spatial=true) {
 const reversal=useRef<{opacity:string;transform:string}|null>(null);
 useLayoutEffect(()=>{
  const node=selector?root.current?.querySelector<HTMLElement>(selector):root.current;
  if(document.documentElement.dataset.swipeNavigation||!ready||!node||node.closest('[hidden]')||matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  let cancel:(()=>void)|undefined,observer:MutationObserver|undefined;
  const enter=()=>{
   // A first lazy visit should reveal the actual page, rather than spend its
   // entrance on the quiet loading placeholder. Stop observing after that paint.
   if([...node.querySelectorAll('.screen-placeholder,.settings-subpage-placeholder')].some(child=>!child.closest('[hidden]')))return;
   const movingContent=spatial&&![...node.querySelectorAll('.track-table')].some(table=>!table.closest('[hidden]'));
   observer?.disconnect();cancel=animateOwned(node,id,[reversal.current??{opacity:'.92',transform:movingContent?'translateY(4px)':'none'},{opacity:1,transform:'none'}],180);reversal.current=null;
  };
  enter();
  if(!cancel){
   observer=new MutationObserver(enter);
   observer.observe(node,{childList:true,subtree:true});
  }
  return()=>{observer?.disconnect();if(cancel){const moving=node.getAnimations().some(animation=>animation.id===id&&animation.playState==='running');const style=moving?getComputedStyle(node):null;reversal.current=style?{opacity:style.opacity,transform:style.transform}:null;cancel();}};
 },[root,location,ready,selector,id]);
}
export function usePageEntrance(root:RefObject<HTMLElement|null>,location:string,ready:boolean) {
 const transfer=coverTransferPending();
 useAlbumReveal(root,location,ready);
 // Detail groups and artwork already animate independently. Fading the whole
 // retained page as well adds a large compositing layer and doubles the fade.
 useContentFade(root,location,ready&&!transfer&&!/^Library:(album|artist):/.test(location),'.page-content',/^Library:(albums|artists)(:|$)/.test(location)?'library-labels':'navigation-content',!location.startsWith('Library:'));
}

export function parallaxOffset(center:number,top:number,height:number) {
 return Math.max(-14,Math.min(14,((center-top)/Math.max(1,height)-.5)*40));
}
export function useArtworkParallax(_root:RefObject<HTMLElement|null>,_scroll:RefObject<HTMLElement|null>,_enabled=true){
 // Scrolling remains native. Artwork moves only through the depth hover layer.
}


// One shared frame loop. Reversing hover retains both displacement and velocity.
export type JellyAnimation={cancel:()=>void;onfinish:(()=>void)|null};
type Spring={element:HTMLElement;x:number;y:number;vx:number;vy:number;last:number;start:number;active:boolean;handle:JellyAnimation};
const springs=new Map<HTMLElement,Spring>();let springFrame=0;
export function springStep(x:number,v:number,target:number,dt:number):[number,number]{
 const w=20,z=.78,a=z*w,b=w*Math.sqrt(1-z*z),d=x-target,e=Math.exp(-a*dt),c=Math.cos(b*dt),s=Math.sin(b*dt),q=(v+a*d)/b;
 return [target+e*(d*c+q*s),e*(v*c-(a*q+b*d)*s)];
}
function tickSprings(now:number){springFrame=0;
 for(const [element,state] of springs){const dt=Math.min(1,Math.max(0,(now-state.last)/1000));state.last=now;
  const pulse=state.active&&now-state.start<85;
  [state.x,state.vx]=springStep(state.x,state.vx,pulse ? .009 : 0,dt);[state.y,state.vy]=springStep(state.y,state.vy,pulse ? -.006 : 0,dt);
  element.style.scale=`${1+Math.max(-.015,Math.min(.015,state.x))} ${1+Math.max(-.015,Math.min(.015,state.y))}`;
  if(!element.isConnected||now-state.start>=440||(!pulse&&Math.abs(state.x)+Math.abs(state.y)+Math.abs(state.vx)+Math.abs(state.vy)<.0005)){
   element.style.removeProperty('scale');springs.delete(element);state.handle.onfinish?.();
  }
 }if(springs.size)springFrame=requestAnimationFrame(tickSprings);
}
export function springScale(element:HTMLElement){const s=springs.get(element);return {x:1+(s?.x??0),y:1+(s?.y??0),moving:!!s};}
export function jelly(element:HTMLElement,active:boolean,_previous?:JellyAnimation){
 const existing=springs.get(element);if(existing){existing.active=active;if(active)existing.start=performance.now();return existing.handle;}
 const handle:JellyAnimation={onfinish:null,cancel:()=>{springs.delete(element);element.style.removeProperty('scale');}};
 const now=performance.now();springs.set(element,{element,x:0,y:0,vx:0,vy:0,last:now,start:now,active,handle});
 if(!springFrame)springFrame=requestAnimationFrame(tickSprings);return handle;
}
export function useJellyMotion(){
 // Controls use 140 ms color/opacity transitions; artwork depth owns its motion.
}
