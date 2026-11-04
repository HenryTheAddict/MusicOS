import {useLayoutEffect,useRef,type RefObject} from 'react';
import {wordProgress} from './lyricWords';

type GlowWord={node:HTMLElement;start:number;end?:number};
/** Mirror only active words outside the scrolling/masked text surface. Geometry
 * is read on word/layout changes; scrolling only translates the cached copies. */
export function useLyricGlow(pane:RefObject<HTMLElement|null>,scroll:RefObject<HTMLDivElement|null>,active:boolean,signature:string,variant:string,content:unknown){
 const tick=useRef<(time:number)=>void>(()=>{}),time=useRef(0),move=useRef<()=>void>(()=>{});
 const api=useRef({tick:(at:number)=>{time.current=at;tick.current(at);},position:()=>move.current()});
 useLayoutEffect(()=>{
  const source=pane.current,viewport=scroll.current;if(!active||!source||!viewport||document.hidden)return;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');if(reduced.matches)return;
  const root=source.closest<HTMLElement>('dialog,.mini-player,.app-shell')??document.body;
  const layer=document.createElement('div');layer.className='lyric-glow-layer';layer.dataset.variant=variant;layer.dataset.following='false';layer.ariaHidden='true';layer.inert=true;root.append(layer);
  let frame=0,originY=viewport.scrollTop,originX=viewport.scrollLeft,words:GlowWord[]=[],copies:{node:HTMLElement;top:number;bottom:number}[]=[],bounds={top:0,bottom:0};
  const update=(at:number)=>{time.current=at;for(const word of words){const progress=wordProgress(word.start,word.end,at);const pulse=word.end===undefined?Math.max(0,1-(at-word.start)/400):Math.sin(Math.PI*progress);word.node.style.setProperty('--glow-strength',String(.08+Math.max(0,pulse)*.42));}};
  tick.current=update;
  const position=()=>{const dy=originY-viewport.scrollTop,dx=originX-viewport.scrollLeft;for(const copy of copies){copy.node.style.transform=`translate(${dx}px,${dy}px)`;copy.node.style.opacity=String(Math.max(0,Math.min(1,(copy.bottom+dy-bounds.top+20)/36,(bounds.bottom-copy.top-dy+20)/36)));}};
  move.current=position;
  const measure=()=>{
   frame=0;if(document.hidden||reduced.matches)return;layer.replaceChildren();words=[];copies=[];originY=viewport.scrollTop;originX=viewport.scrollLeft;
   const base=root.getBoundingClientRect(),box=viewport.getBoundingClientRect(),style=getComputedStyle(source);bounds={top:box.top,bottom:box.bottom};
   for(const key of ['--ink','--accent'])layer.style.setProperty(key,style.getPropertyValue(key));
   for(const line of viewport.querySelectorAll<HTMLElement>('.lyric-line[data-current=true]')){
    if(!line.querySelector('.lyric-word[data-word-active=true]'))continue;observer.observe(line);
    const rect=line.getBoundingClientRect(),copy=line.cloneNode(true) as HTMLElement,css=getComputedStyle(line);copy.className='lyric-glow-clone';copy.removeAttribute('id');copy.removeAttribute('aria-label');copy.removeAttribute('aria-current');copy.tabIndex=-1;copy.inert=true;
    for(const key of ['font-family','font-size','font-weight','font-style','line-height','letter-spacing','text-align','direction','white-space','word-break','overflow-wrap','font-feature-settings','font-kerning','box-sizing','padding-top','padding-right','padding-bottom','padding-left'])copy.style.setProperty(key,css.getPropertyValue(key),'important');
    Object.assign(copy.style,{left:`${rect.left-base.left-root.clientLeft+root.scrollLeft}px`,top:`${rect.top-base.top-root.clientTop+root.scrollTop}px`,width:`${rect.width}px`});
    layer.append(copy);copies.push({node:copy,top:rect.top,bottom:rect.bottom});
    for(const word of copy.querySelectorAll<HTMLElement>('.lyric-word[data-word-active=true]'))words.push({node:word,start:Number(word.dataset.wordStart),end:word.dataset.wordEnd===undefined?undefined:Number(word.dataset.wordEnd)});
   }
   update(time.current);position();
  };
  const request=()=>{if(!frame&&!document.hidden)frame=requestAnimationFrame(measure);};
  const visibility=()=>{if(document.hidden){cancelAnimationFrame(frame);frame=0;layer.replaceChildren();words=[];copies=[];}else request();};
  const motion=()=>{if(reduced.matches){cancelAnimationFrame(frame);frame=0;layer.replaceChildren();words=[];copies=[];}else request();};
  const observer=new ResizeObserver(request);observer.observe(viewport);observer.observe(root);
  viewport.addEventListener('scroll',position,{passive:true});window.addEventListener('resize',request);document.addEventListener('visibilitychange',visibility);document.fonts?.addEventListener('loadingdone',request);reduced.addEventListener('change',motion);measure();
  return()=>{tick.current=()=>{};move.current=()=>{};cancelAnimationFrame(frame);observer.disconnect();viewport.removeEventListener('scroll',position);window.removeEventListener('resize',request);document.removeEventListener('visibilitychange',visibility);document.fonts?.removeEventListener('loadingdone',request);reduced.removeEventListener('change',motion);layer.remove();};
 },[pane,scroll,active,signature,variant,content]);
 return api.current;
}
