import {useCallback,useLayoutEffect,useRef,useState,type RefObject} from 'react';
let focusOwner:HTMLButtonElement|null=null;
export const menuOwnsFocus=(trigger:HTMLButtonElement|null)=>focusOwner===trigger;

/** A stable full-size menu, with a separate growing surface/shadow and clipped items.
 * Nothing scales the text or changes the Popper collision bounds during motion. */
export function useMenuPresentation(trigger:RefObject<HTMLButtonElement|null>){
 const [open,updateOpen]=useState(false),[present,setPresent]=useState(false);
 const [element,attach]=useState<HTMLDivElement|null>(null),content=useRef<HTMLDivElement|null>(null);
 const afterExit=useRef<(()=>void)|null>(null);
 const visible=useRef(false),generation=useRef(0),animations=useRef<Animation[]>([]),frame=useRef(0);
 const ref=useCallback((node:HTMLDivElement|null)=>{content.current=node;attach(node);},[]);
 const setOpen=useCallback((next:boolean)=>{
  if(next){afterExit.current=null;focusOwner=trigger.current;window.dispatchEvent(new CustomEvent('musicos-menu-will-open',{detail:trigger.current}));setPresent(true);}
  visible.current=next;updateOpen(next);
 },[trigger]);
 const closeThen=useCallback((action:()=>void)=>{afterExit.current=action;setOpen(false);},[setOpen]);
 useLayoutEffect(()=>{
  const retire=(event:Event)=>{if((event as CustomEvent).detail!==trigger.current){afterExit.current=null;if(visible.current)setOpen(false);}};
  window.addEventListener('musicos-menu-will-open',retire);
  return()=>window.removeEventListener('musicos-menu-will-open',retire);
 },[setOpen,trigger]);
 useLayoutEffect(()=>{
  if(!element)return;
  const surface=element.querySelector<HTMLElement>(':scope > .menu-panel-surface')!;
  const items=element.querySelector<HTMLElement>(':scope > .menu-panel-items')!;
  const section=element.classList.contains('section-dropdown'),anchor=section?trigger.current:null;
  const fromShadow=anchor?getComputedStyle(anchor).getPropertyValue('--menu-shadow-opacity').trim()||'0':'0';
  const id=++generation.current,wasMoving=animations.current.length>0;
  cancelAnimationFrame(frame.current);
  const fromHeight=surface.getBoundingClientRect().height;
  const fromClip=getComputedStyle(items).clipPath;
  animations.current.forEach(animation=>animation.cancel());animations.current=[];
  surface.style.height=`${fromHeight}px`;items.style.clipPath=fromClip;
  if(anchor){anchor.style.setProperty('--menu-join-height',`${fromHeight}px`);anchor.style.setProperty('--menu-shadow-opacity',fromShadow);}
  const media=typeof matchMedia==='function'?matchMedia('(prefers-reduced-motion: reduce)'):null;
  const finish=()=>{
   if(generation.current!==id)return;
   cancelAnimationFrame(frame.current);animations.current.forEach(animation=>animation.cancel());animations.current=[];
   surface.style.height=visible.current?'100%':'0px';items.style.clipPath=visible.current?'inset(0% 0% 0% 0%)':'inset(0% 0% 100% 0%)';
   if(anchor){if(visible.current){anchor.style.setProperty('--menu-join-height',`${element.getBoundingClientRect().height}px`);anchor.style.setProperty('--menu-shadow-opacity','1');}else{anchor.style.removeProperty('--menu-join-height');anchor.style.removeProperty('--menu-shadow-opacity');}}
   if(!visible.current){setPresent(false);const action=afterExit.current;afterExit.current=null;action?.();}
  };
  if(!media||media.matches||typeof element.animate!=='function'){finish();return;}
  frame.current=requestAnimationFrame(()=>{
   if(generation.current!==id)return;
   const height=element.getBoundingClientRect().height;
   const above=element.dataset.side==='top';
   const collapsed=above?'inset(100% 0% 0% 0%)':'inset(0% 0% 100% 0%)';
   const startClip=wasMoving?fromClip:(open?collapsed:fromClip);
   const distance=open&&fromHeight<=2?1:height?Math.abs((open?height:0)-fromHeight)/height:0;
   const options={duration:140*Math.min(1,distance),easing:section&&!open?'cubic-bezier(.4,0,.6,1)':'cubic-bezier(.2,.75,.2,1)',fill:'both' as FillMode};
   const backdrop=surface.animate([{height:`${fromHeight}px`},{height:open?`${height}px`:'0px'}],options);
   const reveal=items.animate([{clipPath:startClip},{clipPath:open?'inset(0% 0% 0% 0%)':collapsed}],options);
   backdrop.id='menu-surface';reveal.id='menu-reveal';animations.current=[backdrop,reveal];
   if(anchor){const join=anchor.animate([{'--menu-join-height':`${fromHeight}px`,'--menu-shadow-opacity':fromShadow},{'--menu-join-height':open?`${height}px`:'0px','--menu-shadow-opacity':open?'1':'0'}],options);join.id='menu-join';animations.current.push(join);}
   void Promise.all(animations.current.map(animation=>animation.finished)).then(finish).catch(()=>{});
  });
  const reduced=()=>{if(media.matches)finish();};
  window.addEventListener('resize',finish);window.addEventListener('scroll',finish,true);media.addEventListener('change',reduced);
  // Keep the displayed frame until the next effect captures it for a reversal.
  return()=>{window.removeEventListener('resize',finish);window.removeEventListener('scroll',finish,true);media.removeEventListener('change',reduced);};
 },[open,element]);
 useLayoutEffect(()=>()=>{afterExit.current=null;trigger.current?.style.removeProperty('--menu-join-height');trigger.current?.style.removeProperty('--menu-shadow-opacity');generation.current++;cancelAnimationFrame(frame.current);animations.current.forEach(animation=>animation.cancel());animations.current=[];},[]);
 return {open,present,setOpen,closeThen,ref,content,element};
}
