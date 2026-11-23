import {useEffect,useRef,type RefObject} from 'react';
import {flushSync} from 'react-dom';
import {listen} from '@tauri-apps/api/event';
import {native} from './bridge';
import {swipeCover} from './swipeCover';
import type {Palette} from './types';
// A small stack of frozen pages is enough for Back. Keep their old subscriptions out of it.
function locationKey(raw:string){try{const r=JSON.parse(raw);return `${r.view}:${r.id??''}:${r.search??''}`;}catch{return raw}}
export type BackArtwork={request:string;result:{palette:Palette;path:string}|null};
const swipeColors=['shell-color','page-color','surface-color','blob-color','accent'] as const;
const pageThemeColors=[...swipeColors,'ink','secondary-ink','accent-ink','playback-tone'] as const;
type BackTheme={key:string;artwork:BackArtwork;colors:Record<typeof pageThemeColors[number],string>};
type Snapshot = {
 html:string;
 scroll:number;
 theme?:BackTheme;
 frame:{left:number;top:number;right:number;bottom:number};
 size:number;
};
const snapshots=new Map<string,Snapshot>();
let total=0;

function freezeMarkup(source:HTMLElement){
 const copy=source.cloneNode(false) as HTMLElement;
 const content=source.querySelector(':scope > .page-content');
 const sourceImages:HTMLImageElement[]=[];
 if(content){
  // Home, search and settings can still be mounted while hidden.
  // Copy the page on screen and leave the rest alone.
  const visible=content.cloneNode(false);
  for(const child of content.children){
   if(child.matches('[hidden]'))continue;
   visible.appendChild(child.cloneNode(true));
   sourceImages.push(...child.querySelectorAll<HTMLImageElement>('img'));
  }
  copy.appendChild(visible);
 }else{
  for(const child of source.childNodes)copy.appendChild(child.cloneNode(true));
  sourceImages.push(...source.querySelectorAll<HTMLImageElement>('img'));
 }
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
  for(const key of node.getAttributeNames()){
   if(key==='id')continue;
   const value=node.getAttribute(key)!;
   if(!value.includes('#'))continue;
   const next=value.replace(/url\(#([^)]+)\)/g,(_,id)=>`url(#${ids.get(id)??id})`);
   const updated=next.startsWith('#')&&ids.has(next.slice(1))?'#'+ids.get(next.slice(1)):next;
   if(updated!==value)node.setAttribute(key,updated);
  }
 }
 return copy.outerHTML;
}

export function captureBackPage(root:HTMLElement|null,key:string,themeKey:string,artwork?:BackArtwork){
 const column=root?.closest<HTMLElement>('.content-column');
 const frame=root?.parentElement;
 if(!root||!column||!frame)return;
 key=locationKey(key);
 const columnBox=column.getBoundingClientRect(),frameBox=frame.getBoundingClientRect();
 const html=freezeMarkup(root);
 // Save the resolved endpoint, even if the previous page is still tinting.
 const style=document.documentElement.style;
 const theme=themeKey&&artwork?{
  key:themeKey,artwork,
  colors:Object.fromEntries(pageThemeColors.map(name=>[name,style.getPropertyValue(`--${name}`)])) as BackTheme['colors'],
 }:undefined;
 const size=html.length;
 total-=snapshots.get(key)?.size??0;
 snapshots.delete(key);
 snapshots.set(key,{html,theme,scroll:root.scrollTop,size,
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
 return {frame,page,scroll:saved.scroll};
}

export const backSwipeThreshold=110;
export function acceptsBackAxis(x:number,y:number){return x< -10&&Math.abs(x)>Math.abs(y)*1.8;}
export function consumesHorizontalScroll(target:Element|null,root:Element){for(let node=target;node&&node!==root;node=node.parentElement){if(node.matches('input,textarea,select,[contenteditable=true],[role=slider],dialog,[role=menu],.queue-scroll,.mini-queue-list'))return true;const style=getComputedStyle(node);if(/auto|scroll/.test(style.overflowX)&&node.scrollWidth>node.clientWidth+2)return true;}return false;}
export function shouldCommitSwipe(distance:number,width:number,velocity:number){return distance>=width*.25||(distance>=60&&velocity>=.55);}
export function useBackSwipe(root:RefObject<HTMLElement|null>,enabled:boolean,canBack:boolean,onBack:(interactive:boolean,artwork?:BackArtwork)=>void,previousKey='',themeKey=''){
 const latest=useRef({enabled,canBack,onBack,previousKey,themeKey});
 latest.current={enabled,canBack,onBack,previousKey,themeKey};
 const cue=useRef<HTMLDivElement>(null);
 const prepared=useRef<{saved:Snapshot;preview:ReturnType<typeof preparePreview>}|null>(null);
 useEffect(()=>{
  const key=locationKey(previousKey);
  prepared.current=null;
  const job=window.setTimeout(()=>{
   const saved=snapshots.get(key);
   if(saved)prepared.current={saved,preview:preparePreview(saved)};
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
 let animation:Animation|null=null,fingers=false,momentum=false,ending=false,disposed=false,unlisten:(()=>void)|undefined;
 let release=0;
 const inertBefore=page.inert;
 let cover:ReturnType<typeof swipeCover>;
 const pageColors=new Map<string,string>();
 const blocked=()=>!latest.current.enabled||!latest.current.canBack||!!document.querySelector('dialog[open],[role=menu]:not([inert]),.quick-audio:popover-open');
 const setProgress=(progress:number)=>themeRoot.style.setProperty('--back-swipe-progress',String(progress));
 const clearTheme=()=>{
  if(!theme)return;
  for(const [color,value] of pageColors){
   if(value)page.style.setProperty(color,value);else page.style.removeProperty(color);
  }
  pageColors.clear();
  for(const name of pageThemeColors)preview?.frame.style.removeProperty(`--${name}`);
  delete themeRoot.dataset.backSwipeTheme;
  for(const color of swipeColors){
   themeRoot.style.removeProperty(`--swipe-from-${color}`);
   themeRoot.style.removeProperty(`--swipe-to-${color}`);
  }
  theme=undefined;
 };
 const clear=()=>{
  cancelAnimationFrame(handoffFrame);
  cover?.release();cover=undefined;
  animation?.cancel();animation=null;
  clearTheme();
  preview?.frame.remove();preview=undefined;
  themeRoot.style.removeProperty('--back-swipe-progress');
  themeRoot.style.removeProperty('--back-swipe-duration');
  page.style.removeProperty('--back-swipe-offset');
  delete column.dataset.backSwipe;
  delete document.documentElement.dataset.swipeNavigation;
  page.inert=inertBefore;
  delete page.dataset.swiping;
 };
 const paint=()=>{
  frame=0;
  if(!preview)return;
  if(theme&&theme.key!==latest.current.themeKey)clearTheme();
  const progress=Math.max(0,Math.min(1,-amount/Math.max(1,width)));
  page.style.setProperty('--back-swipe-offset',`${progress*width}px`);
  cover?.seek(progress);
 };
 const reset=()=>{release++;ending=false;clearTimeout(timer);cancelAnimationFrame(frame);clear();x=0;y=0;axis='pending';locked=false;velocity=0;peakVelocity=0;peakAt=0;motionAt=0;sample=0;amount=0;};
 const cancel=()=>{release++;ending=false;fingers=false;clearTimeout(timer);cancelAnimationFrame(frame);clear();locked=true;axis='ignored';};
 const finish=(commit:boolean)=>{
  clearTimeout(timer);timer=0;
  if(axis!=='horizontal'||locked)return;
  locked=true;
  ending=false;
  const owner=++release;
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
   if(disposed||owner!==release)return;
   if(!commit){clear();return;}
   if(theme&&theme.key!==latest.current.themeKey)clearTheme();
   setProgress(1);
   column.dataset.backSwipe='handoff';
   document.documentElement.dataset.swipeNavigation='true';
   flushSync(()=>latest.current.onBack(true,theme?.artwork));
   // Settle the colors before letting transitions run again.
   // Doing both at once gives the shell another unwanted tint change.
   if(theme&&swipeColors.every(color=>themeRoot.style.getPropertyValue(`--${color}`)===theme?.colors[color])){
    themeRoot.dataset.backSwipeTheme='handoff';
   }
   // Keep the preview up while the real page finds its place.
   delete page.dataset.swiping;
   animation?.cancel();animation=null;
   const deadline=performance.now()+260;
   const swap=()=>{
    if(disposed||owner!==release)return;
    if(cover&&!cover.destinationPaintable()&&performance.now()<deadline){
     handoffFrame=requestAnimationFrame(swap);
     return;
    }
    // Keep the cover here until the real tile can paint.
    // Removing it early gives us that one-frame flash again.
    handoffFrame=requestAnimationFrame(clear);
   };
   handoffFrame=requestAnimationFrame(swap);
  };
  // A fast swipe can finish moving in a blink.
  // Give the palette a little time to catch up without flashing.
  const floor=theme&&commit?128:0;
  const duration=commit?Math.max(floor,180*(1-progress)):Math.min(140,70+progress*140);
  if(duration<8){if(commit)cover?.seek(1);done();return;}
  const from=progress*width,to=commit?width:0;
  animation=page.animate([{transform:`translate3d(${from}px,0,0)`},{transform:`translate3d(${to}px,0,0)`}],
    {duration,easing:'cubic-bezier(.2,.7,.2,1)',fill:'forwards'});
  if(theme){
   themeRoot.dataset.backSwipeTheme='release';
   themeRoot.style.setProperty('--back-swipe-duration',`${duration}ms`);
   setProgress(commit?1:0);
  }
  animation.id='interactive-back-finish';
  void Promise.all([animation.finished,cover?.finish(commit,duration)]).then(done).catch(()=>{});
 };
 const beginPreview=()=>{
  width=page.getBoundingClientRect().width;
  const key=locationKey(latest.current.previousKey),cached=prepared.current,saved=snapshots.get(key);
  if(!saved||media.matches)return;
  preview=cached?.saved===saved?cached.preview:preparePreview(saved);
  // Fresh swipe, fresh copy. The last gesture may have moved things around.
  prepared.current=null;
  column.append(preview.frame);
  preview.page.scrollTop=preview.scroll;
  cover=swipeCover(page,preview.page);
  if(!cover)window.dispatchEvent(new Event('musicos-back-gesture-start'));
  const targetTheme=saved.theme;
  if(targetTheme&&targetTheme.key===latest.current.themeKey){
   theme=targetTheme;
   for(const [name,value] of Object.entries(targetTheme.colors))preview.frame.style.setProperty(`--${name}`,value);
   const displayed=getComputedStyle(themeRoot);
   const outgoing=getComputedStyle(page);
   for(const name of pageThemeColors){
    const property=`--${name}`;
    pageColors.set(property,page.style.getPropertyValue(property));
    page.style.setProperty(property,outgoing.getPropertyValue(property));
   }
   const from=swipeColors.map(color=>displayed.getPropertyValue(`--${color}`));
   swipeColors.forEach((color,index)=>{
    themeRoot.style.setProperty(`--swipe-from-${color}`,from[index]);
    themeRoot.style.setProperty(`--swipe-to-${color}`,targetTheme.colors[color]);
   });
   themeRoot.dataset.backSwipeTheme='true';
  }
  column.dataset.backSwipe='dragging';
  page.dataset.swiping='true';
  page.inert=true;
  amount=x;
  paint();
  page.getAnimations({subtree:true}).filter(a=>a.id.includes('reveal')||a.id.includes('navigation')).forEach(a=>a.cancel());
 };
 const releaseVelocity=()=>performance.now()-motionAt>120?0:performance.now()-peakAt<100?Math.max(velocity,peakVelocity):velocity;
 const settle=(delay:number)=>{
  clearTimeout(timer);
  timer=window.setTimeout(()=>{
   fingers=false;
   ending=false;
   finish(shouldCommitSwipe(-x,width,releaseVelocity()));
  },delay);
 };
 const wheel=(event:WheelEvent)=>{
  if(event.defaultPrevented||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||event.deltaMode!==0||blocked())return;
  if(momentum)return;
  const now=performance.now();
  if(now-last>280&&!fingers&&!ending)reset();
  const dt=now-(sample||now-16);
  last=now;sample=now;
  if(locked||axis==='ignored')return;
  // An inert outgoing page can change the wheel target once the drag starts.
  if(axis==='pending'&&(consumesHorizontalScroll(event.target as Element,host)
    ||(event.target as Element|null)?.closest('.audio-panel'))){axis='ignored';return;}
  x+=event.deltaX;y+=event.deltaY;
  if(event.deltaX>0){velocity=0;peakVelocity=0;peakAt=now;motionAt=now;}
  else if(event.deltaX<0){
   const speed=-event.deltaX/Math.max(1,dt);
   velocity=velocity*.35+speed*.65;motionAt=now;
   if(speed>=peakVelocity||now-peakAt>100){peakVelocity=speed;peakAt=now;}
  }
  if(axis==='pending'){
   if(Math.abs(x)<12&&Math.abs(y)<12)return;
   if(!acceptsBackAxis(x,y)){axis='ignored';return;}
   axis='horizontal';beginPreview();
  }
  event.preventDefault();
  if(Math.abs(y)>Math.abs(x)*.8||x>=0){finish(false);return;}
  amount=x;
  if(!frame)frame=requestAnimationFrame(paint);
  if(!ending)settle(fingers?500:80);
 };
 const phase=(value:number)=>{
  if(value===1){
   momentum=false;fingers=true;ending=false;
   clearTimeout(timer);
   const now=performance.now();
   if(animation&&axis==='horizontal'&&column.dataset.backSwipe!=='handoff'){
    const transform=getComputedStyle(page).transform;
    const offset=transform==='none'?0:new DOMMatrixReadOnly(transform).m41;
    release++;animation.cancel();animation=null;locked=false;
    x=-offset;amount=x;y=0;velocity=0;peakVelocity=0;sample=0;
    if(theme){
     const style=getComputedStyle(themeRoot);
     const colors=swipeColors.map(color=>style.getPropertyValue(`--${color}`));
     swipeColors.forEach((color,index)=>themeRoot.style.setProperty(`--swipe-from-${color}`,colors[index]));
     themeRoot.dataset.backSwipeTheme='true';setProgress(0);
    }
    paint();
   }else if(locked||axis==='ignored'||now-last>50)reset();
   last=now;
  }else if(value===2||value===4){
   // AppKit can announce the release before WebKit delivers the last wheel event.
   // Give that final sample a frame to catch up.
   fingers=false;ending=true;
   if(value===4)momentum=true;
   settle(24);
  }else if(value===3){fingers=false;ending=false;finish(false);}
  else if(value===5){momentum=false;last=performance.now();}
 };
 if(native)void listen<number>('trackpad-phase',event=>phase(event.payload)).then(off=>{if(disposed)off();else unlisten=off;});const testPhase=(event:Event)=>phase((event as CustomEvent<number>).detail);window.addEventListener('musicos-trackpad-phase',testPhase);
 host.addEventListener('wheel',wheel,{passive:false});window.addEventListener('resize',cancel);window.addEventListener('blur',cancel);window.addEventListener('pointerdown',cancel);const navigation=()=>{if(!document.documentElement.dataset.swipeNavigation)cancel();};window.addEventListener('musicos-navigation-start',navigation);media.addEventListener('change',cancel);
 return()=>{disposed=true;unlisten?.();host.removeEventListener('wheel',wheel);window.removeEventListener('resize',cancel);window.removeEventListener('blur',cancel);window.removeEventListener('pointerdown',cancel);window.removeEventListener('musicos-navigation-start',navigation);window.removeEventListener('musicos-trackpad-phase',testPhase);media.removeEventListener('change',cancel);clearTimeout(timer);cancelAnimationFrame(frame);clear();};
 },[root,enabled]);return cue;
}
