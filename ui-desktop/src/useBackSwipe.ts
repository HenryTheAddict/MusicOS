import {useEffect,useRef,type RefObject} from 'react';
import {flushSync} from 'react-dom';
import {listen} from '@tauri-apps/api/event';
import {native} from './bridge';
import {swipeCover} from './swipeCover';
import type {Palette} from './types';
// History retains bounded inert markup, never live page components or subscriptions.
function locationKey(raw:string){try{const r=JSON.parse(raw);return `${r.view}:${r.id??''}:${r.search??''}`;}catch{return raw}}
export type BackArtwork={request:string;result:{palette:Palette;path:string}|null};
const swipeColors=['shell-color','page-color','surface-color','blob-color','accent'] as const;
type BackTheme={appearance:string;artwork:BackArtwork;colors:Record<typeof swipeColors[number],string>};
type Bounds = {left:number;top:number;width:number;height:number};
type Snapshot = {
 html:string;
 scroll:number;
 theme?:BackTheme;
 columnWidth:number;
 frame:{left:number;top:number;right:number;bottom:number};
 bars:{html:string;bounds:Bounds}[];
 size:number;
};
const snapshots=new Map<string,Snapshot>();
let total=0;
const barSelector=':scope > .library-toolbar:not(.library-toolbar-exit,.back-swipe-bar), :scope > .list-find[data-open=true]:not(.back-swipe-bar)';

function freezeMarkup(source:HTMLElement){
 const copy=source.cloneNode(true) as HTMLElement;
 const sourceImages=source.querySelectorAll<HTMLImageElement>('img');
 copy.querySelectorAll<HTMLImageElement>('img').forEach((image,index)=>{
  const original=sourceImages[index];
  if(original?.complete&&original.naturalWidth){
   image.src=original.currentSrc||original.src;
   image.loading='eager';
   image.removeAttribute('srcset');
  }else image.remove();
 });
 copy.removeAttribute('id');
 copy.removeAttribute('data-swiping');
 copy.querySelectorAll('video,audio,source,iframe,object,embed,script,canvas,dialog,[role=menu],.find-help,[hidden]').forEach(node=>node.remove());
 const ids=new Map<string,string>(),prefix=`back-${Math.random().toString(36).slice(2)}-`;
 copy.querySelectorAll('[id]').forEach(node=>{ids.set(node.id,prefix+node.id);node.id=prefix+node.id;});
 for(const node of [copy,...copy.querySelectorAll<HTMLElement>('*')]){
  node.style.setProperty('animation','none','important');
  node.style.setProperty('transition','none','important');
  for(const key of node.getAttributeNames()){
   if(key==='id')continue;
   const value=node.getAttribute(key)!;
   const next=value.replace(/url\(#([^)]+)\)/g,(_,id)=>`url(#${ids.get(id)??id})`);
   const updated=next.startsWith('#')&&ids.has(next.slice(1))?'#'+ids.get(next.slice(1)):next;
   if(updated!==value)node.setAttribute(key,updated);
  }
 }
 return copy.outerHTML;
}

export function captureBackPage(root:HTMLElement|null,key:string,appearance:string,artwork?:BackArtwork){
 const column=root?.closest<HTMLElement>('.content-column');
 const frame=root?.parentElement;
 if(!root||!column||!frame)return;
 key=locationKey(key);
 const columnBox=column.getBoundingClientRect(),frameBox=frame.getBoundingClientRect();
 const bars=[...column.querySelectorAll<HTMLElement>(barSelector)].map(bar=>{
  const {left,top,width,height}=bar.getBoundingClientRect();
  return {html:freezeMarkup(bar),bounds:{left:left-columnBox.left,top:top-columnBox.top,width,height}};
 });
 const html=freezeMarkup(root);
 // Save the resolved endpoint, even if the previous page is still tinting.
 const style=document.documentElement.style;
 const theme=appearance&&artwork?{
  appearance,artwork,
  colors:Object.fromEntries(swipeColors.map(name=>[name,style.getPropertyValue(`--${name}`)])) as BackTheme['colors'],
 }:undefined;
 const size=html.length+bars.reduce((sum,bar)=>sum+bar.html.length,0);
 total-=snapshots.get(key)?.size??0;
 snapshots.delete(key);
 snapshots.set(key,{html,bars,theme,scroll:root.scrollTop,columnWidth:columnBox.width,size,
  frame:{left:frameBox.left-columnBox.left,top:frameBox.top-columnBox.top,right:columnBox.right-frameBox.right,bottom:columnBox.bottom-frameBox.bottom}});
 total+=size;
 while(snapshots.size>8||total>4_000_000){
  const first=snapshots.keys().next().value!;
  total-=snapshots.get(first)!.size;
  snapshots.delete(first);
 }
}

function preparePreview(saved:Snapshot){
 const frame=document.createElement('div');
 frame.className='page-frame back-page-preview';
 frame.inert=true;
 frame.ariaHidden='true';
 const {top,right,bottom,left}=saved.frame;
 frame.style.inset=`${top}px ${right}px ${bottom}px ${left}px`;
 frame.innerHTML=saved.html;
 const page=frame.firstElementChild as HTMLElement;
 Object.assign(page.style,{width:'100%',height:'100%',transform:'none',margin:'0'});
 const bars=saved.bars.map(({html,bounds})=>{
  const template=document.createElement('template');
  template.innerHTML=html;
  const bar=template.content.firstElementChild as HTMLElement;
  bar.classList.add('back-swipe-bar');
  bar.inert=true;
  bar.ariaHidden='true';
  for(const [name,value] of Object.entries(bounds))bar.style.setProperty(name,`${value}px`);
  return bar;
 });
 return {frame,page,bars,scroll:saved.scroll};
}

export const backSwipeThreshold=110;
export function acceptsBackAxis(x:number,y:number){return x< -10&&Math.abs(x)>Math.abs(y)*1.8;}
export function consumesHorizontalScroll(target:Element|null,root:Element){for(let node=target;node&&node!==root;node=node.parentElement){if(node.matches('input,textarea,select,[contenteditable=true],[role=slider],dialog,[role=menu],.queue-scroll,.mini-queue-list'))return true;const style=getComputedStyle(node);if(/auto|scroll/.test(style.overflowX)&&node.scrollWidth>node.clientWidth+2)return true;}return false;}
export function shouldCommitSwipe(distance:number,width:number,velocity:number){return distance>=width*.25||(distance>=60&&velocity>=.55);}
export function useBackSwipe(root:RefObject<HTMLElement|null>,enabled:boolean,canBack:boolean,onBack:(interactive:boolean,artwork?:BackArtwork)=>void,previousKey='',appearance=''){
 const latest=useRef({enabled,canBack,onBack,previousKey,appearance});
 latest.current={enabled,canBack,onBack,previousKey,appearance};
 const cue=useRef<HTMLDivElement>(null);
 const prepared=useRef<{key:string;preview:ReturnType<typeof preparePreview>}|null>(null);
 useEffect(()=>{
  const key=locationKey(previousKey);
  prepared.current=null;
  const job=window.setTimeout(()=>{
   const saved=snapshots.get(key);
   if(saved)prepared.current={key,preview:preparePreview(saved)};
  },0);
  return()=>clearTimeout(job);
 },[previousKey]);
 useEffect(()=>{
 const host=root.current;
 if(!host||!enabled)return;
 const page=host.querySelector<HTMLElement>('.page-scroll');
 const column=page?.closest<HTMLElement>('.content-column');
 if(!page||!column)return;
 const media=matchMedia('(prefers-reduced-motion: reduce)');
 const themeRoot=document.documentElement;
 let x=0,y=0,last=0,sample=0,velocity=0,peakVelocity=0,peakAt=0,motionAt=0,width=0;
 let axis:'pending'|'horizontal'|'ignored'='pending';
 let locked=false,timer=0,frame=0,amount=0,handoffFrame=0;
 let preview:ReturnType<typeof preparePreview>|undefined,theme:BackTheme|undefined;
 let animation:Animation|null=null,fingers=false,momentum=false,disposed=false,unlisten:(()=>void)|undefined;
 const inertBefore=page.inert;
 let cover:ReturnType<typeof swipeCover>;
 const blocked=()=>!latest.current.enabled||!latest.current.canBack||!!document.querySelector('dialog[open],[role=menu]:not([inert]),.audio-panel');
 const setProgress=(progress:number)=>themeRoot.style.setProperty('--back-swipe-progress',String(progress));
 const clear=()=>{
  cancelAnimationFrame(handoffFrame);
  cover?.release();cover=undefined;
  animation?.cancel();animation=null;
  preview?.frame.remove();preview?.bars.forEach(bar=>bar.remove());preview=undefined;
  themeRoot.style.removeProperty('--back-swipe-progress');
  if(theme){
   delete themeRoot.dataset.backSwipeTheme;
   for(const color of swipeColors){
    themeRoot.style.removeProperty(`--swipe-from-${color}`);
    themeRoot.style.removeProperty(`--swipe-to-${color}`);
   }
   theme=undefined;
  }
  column.style.removeProperty('--back-swipe-width');
  delete column.dataset.backSwipe;
  delete document.documentElement.dataset.swipeNavigation;
  page.inert=inertBefore;
  delete page.dataset.swiping;
 };
 const paint=()=>{
  frame=0;
  if(!preview)return;
  const progress=Math.max(0,Math.min(1,-amount/Math.max(1,width)));
  setProgress(progress);
  cover?.seek(progress);
 };
 const reset=()=>{clearTimeout(timer);cancelAnimationFrame(frame);clear();x=0;y=0;axis='pending';locked=false;velocity=0;peakVelocity=0;peakAt=0;motionAt=0;sample=0;amount=0;};
 const cancel=()=>{clearTimeout(timer);cancelAnimationFrame(frame);clear();locked=true;axis='ignored';};
 const finish=(commit:boolean)=>{
  clearTimeout(timer);timer=0;
  if(axis!=='horizontal'||locked)return;
  locked=true;
  commit=commit&&!blocked();
  cancelAnimationFrame(frame);
  if(!preview){
   clear();
   if(commit)flushSync(()=>latest.current.onBack(false));
   return;
  }
  paint();
  const progress=Math.min(1,Math.max(0,-amount/width));
  const done=()=>{
   if(disposed)return;
   if(!commit){clear();return;}
   setProgress(1);
   column.dataset.backSwipe='handoff';
   document.documentElement.dataset.swipeNavigation='true';
   flushSync(()=>latest.current.onBack(true,theme?.artwork));
   // Switch to the equivalent native color values while transitions are held.
   // Removing color-mix() and enabling transitions together would retint the shell.
   if(theme&&swipeColors.every(color=>themeRoot.style.getPropertyValue(`--${color}`)===theme?.colors[color])){
    themeRoot.dataset.backSwipeTheme='handoff';
   }
   // Keep the destination visible while the live page restores layout and paints.
   delete page.dataset.swiping;
   animation?.cancel();animation=null;
   handoffFrame=requestAnimationFrame(()=>{handoffFrame=requestAnimationFrame(clear);});
  };
  const duration=commit?160*(1-progress):Math.min(140,70+progress*140);
  if(duration<16){if(commit)cover?.seek(1);done();return;}
  animation=themeRoot.animate([{'--back-swipe-progress':progress},{'--back-swipe-progress':commit?1:0}],{duration,easing:'cubic-bezier(.2,.7,.2,1)',fill:'forwards'});
  animation.id='interactive-back-finish';
  void Promise.all([animation.finished,cover?.finish(commit,duration)]).then(done).catch(()=>{});
 };
 const beginPreview=()=>{
  width=page.getBoundingClientRect().width;
  const key=locationKey(latest.current.previousKey),cached=prepared.current,saved=snapshots.get(key);
  if(!saved||media.matches||Math.abs(saved.columnWidth-column.getBoundingClientRect().width)>1)return;
  preview=cached?.key===key?cached.preview:preparePreview(saved);
  // Each gesture owns its copy, so reversing or restarting never reuses altered markup.
  prepared.current=null;
  column.append(preview.frame,...preview.bars);
  preview.page.scrollTop=preview.scroll;
  cover=swipeCover(page,preview.page);
  if(!cover)window.dispatchEvent(new Event('musicos-back-gesture-start'));
  const targetTheme=saved.theme;
  if(targetTheme&&targetTheme.appearance===latest.current.appearance){
   theme=targetTheme;
   const displayed=getComputedStyle(themeRoot);
   const from=swipeColors.map(color=>displayed.getPropertyValue(`--${color}`));
   swipeColors.forEach((color,index)=>{
    themeRoot.style.setProperty(`--swipe-from-${color}`,from[index]);
    themeRoot.style.setProperty(`--swipe-to-${color}`,targetTheme.colors[color]);
   });
   themeRoot.dataset.backSwipeTheme='true';
  }
  column.style.setProperty('--back-swipe-width',`${width}px`);
  column.dataset.backSwipe='dragging';
  page.dataset.swiping='true';
  page.inert=true;
  amount=x;
  paint();
  page.getAnimations({subtree:true}).filter(a=>a.id.includes('reveal')||a.id.includes('navigation')).forEach(a=>a.cancel());
 };
 const releaseVelocity=()=>performance.now()-motionAt>120?0:performance.now()-peakAt<100?Math.max(velocity,peakVelocity):velocity;
 const wheel=(event:WheelEvent)=>{if(event.defaultPrevented||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||event.deltaMode!==0||blocked())return;const now=performance.now();if(momentum)return;if(now-last>280&&!fingers)reset();const dt=now-(sample||now-16);last=now;sample=now;if(locked||axis==='ignored')return;if(consumesHorizontalScroll(event.target as Element,host)){axis='ignored';return;}x+=event.deltaX;y+=event.deltaY;if(event.deltaX>0){velocity=0;peakVelocity=0;peakAt=now;motionAt=now;}else if(event.deltaX<0){const speed=-event.deltaX/Math.max(1,dt);velocity=velocity*.35+speed*.65;motionAt=now;if(speed>=peakVelocity||now-peakAt>100){peakVelocity=speed;peakAt=now;}}if(axis==='pending'){if(Math.abs(x)<12&&Math.abs(y)<12)return;if(!acceptsBackAxis(x,y)){axis='ignored';return;}axis='horizontal';beginPreview();}if(Math.abs(y)>Math.abs(x)*.8||x>0){finish(false);return;}event.preventDefault();amount=x;if(!frame)frame=requestAnimationFrame(paint);clearTimeout(timer);if(!fingers)timer=window.setTimeout(()=>finish(shouldCommitSwipe(-x,width,releaseVelocity())),80);};
 const phase=(value:number)=>{if(value===1){momentum=false;fingers=true;const now=performance.now();if(animation&&axis==='horizontal'){const progress=Number(getComputedStyle(themeRoot).getPropertyValue('--back-swipe-progress'));setProgress(progress);animation.cancel();animation=null;locked=false;x=-progress*width;amount=x;y=0;velocity=0;peakVelocity=0;sample=0;cover?.seek(progress);}else if(locked||axis==='ignored'||now-last>50)reset();last=now;}else if(value===2){fingers=false;finish(shouldCommitSwipe(-x,width,releaseVelocity()));}else if(value===3){fingers=false;finish(false);}else if(value===4){fingers=false;momentum=true;if(axis==='horizontal'&&!locked)finish(shouldCommitSwipe(-x,width,releaseVelocity()));locked=true;}else if(value===5){momentum=false;last=performance.now();}};
 if(native)void listen<number>('trackpad-phase',event=>phase(event.payload)).then(off=>{if(disposed)off();else unlisten=off;});const testPhase=(event:Event)=>phase((event as CustomEvent<number>).detail);window.addEventListener('musicos-trackpad-phase',testPhase);
 host.addEventListener('wheel',wheel,{passive:false});window.addEventListener('resize',cancel);window.addEventListener('blur',cancel);window.addEventListener('pointerdown',cancel);const navigation=()=>{if(!document.documentElement.dataset.swipeNavigation)cancel();};window.addEventListener('musicos-navigation-start',navigation);media.addEventListener('change',cancel);
 return()=>{disposed=true;unlisten?.();host.removeEventListener('wheel',wheel);window.removeEventListener('resize',cancel);window.removeEventListener('blur',cancel);window.removeEventListener('pointerdown',cancel);window.removeEventListener('musicos-navigation-start',navigation);window.removeEventListener('musicos-trackpad-phase',testPhase);media.removeEventListener('change',cancel);clearTimeout(timer);cancelAnimationFrame(frame);clear();};
 },[root,enabled]);return cue;
}
