import {useCallback,useLayoutEffect,useRef,useState,type RefObject} from 'react';
let focusOwner:{trigger:HTMLButtonElement|null;session:number}|null=null;
let nextSession=0;
export const menuOwnsFocus=(trigger:HTMLButtonElement|null,session:number)=>focusOwner?.trigger===trigger&&focusOwner.session===session;

/** Stable collision bounds and unscaled text. The same timeline runs forwards
 * and backwards, so reversal preserves both its displayed frame and easing. */
export function useMenuPresentation(trigger:RefObject<HTMLButtonElement|null>){
 const [open,updateOpen]=useState(false),[present,setPresent]=useState(false),[session,setSession]=useState(0);
 const [element,attach]=useState<HTMLDivElement|null>(null),content=useRef<HTMLDivElement|null>(null);
 const afterExit=useRef<(()=>void)|null>(null);
 const visible=useRef(false),generation=useRef(0),animations=useRef<Animation[]>([]),frame=useRef(0);
 const ref=useCallback((node:HTMLDivElement|null)=>{content.current=node;attach(node);},[]);
 const setOpen=useCallback((next:boolean)=>{
  if(next){
   afterExit.current=null;
   const session=++nextSession;focusOwner={trigger:trigger.current,session};setSession(session);
   window.dispatchEvent(new CustomEvent('musicos-menu-will-open',{detail:trigger.current}));setPresent(true);
  }
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
  const id=++generation.current;
  cancelAnimationFrame(frame.current);
  const media=typeof matchMedia==='function'?matchMedia('(prefers-reduced-motion: reduce)'):null;
  const finish=()=>{
   if(generation.current!==id)return;
   cancelAnimationFrame(frame.current);
   // Commit the endpoint before removing animation effects. This also avoids a
   // transparent/border-only frame at the final WebKit composite.
   surface.style.height=visible.current?'100%':'0px';items.style.clipPath=visible.current?'inset(0% 0% 0% 0%)':'inset(0% 0% 100% 0%)';
   if(anchor){if(visible.current){anchor.style.setProperty('--menu-join-height',`${element.getBoundingClientRect().height}px`);anchor.style.setProperty('--menu-shadow-opacity','1');}else{anchor.style.removeProperty('--menu-join-height');anchor.style.removeProperty('--menu-shadow-opacity');}}
   animations.current.forEach(animation=>animation.cancel());animations.current=[];
   if(!visible.current){setPresent(false);const action=afterExit.current;afterExit.current=null;action?.();}
  };
  const play=()=>{
   for(const animation of animations.current){animation.playbackRate=open?1:-1;animation.play();}
   void Promise.all(animations.current.map(animation=>animation.finished)).then(finish).catch(()=>{});
  };
  if(!media||media.matches||typeof element.animate!=='function')finish();
  else if(animations.current.length)play();
  else frame.current=requestAnimationFrame(()=>{
   if(generation.current!==id)return;
   const height=element.getBoundingClientRect().height;
   const above=element.dataset.side==='top';
   const collapsed=above?'inset(100% 0% 0% 0%)':'inset(0% 0% 100% 0%)';
   const options={duration:140,easing:'cubic-bezier(.3,0,.7,1)',fill:'both' as FillMode};
   const backdrop=surface.animate([{height:'0px'},{height:`${height}px`}],options);
   const reveal=items.animate([{clipPath:collapsed},{clipPath:'inset(0% 0% 0% 0%)'}],options);
   backdrop.id='menu-surface';reveal.id='menu-reveal';animations.current=[backdrop,reveal];
   if(anchor){const join=anchor.animate([{'--menu-join-height':'0px','--menu-shadow-opacity':'0'},{'--menu-join-height':`${height}px`,'--menu-shadow-opacity':'1'}],options);join.id='menu-join';animations.current.push(join);}
   animations.current.forEach(animation=>{animation.pause();animation.currentTime=open?0:140;});
   play();
  });
  const reduced=()=>{if(media?.matches)finish();};
  // Scrolling within a panel must not snap its animation. Outside scrolling
  // dismisses through the same exit, including when an exit is already running.
  const anchorBox=trigger.current?.getBoundingClientRect();
  const scroll=(event:Event)=>{if(!visible.current||event.target instanceof Node&&element.contains(event.target))return;const current=trigger.current?.getBoundingClientRect();
   // A queued scroll from bringing the trigger into view predates its opening.
   // Dismiss only when subsequent scrolling actually moves the attachment.
   if(!anchorBox||!current||Math.abs(current.x-anchorBox.x)>.5||Math.abs(current.y-anchorBox.y)>.5)setOpen(false);
  };
  window.addEventListener('resize',finish);window.addEventListener('scroll',scroll,true);media?.addEventListener('change',reduced);
  return()=>{window.removeEventListener('resize',finish);window.removeEventListener('scroll',scroll,true);media?.removeEventListener('change',reduced);};
 },[open,element,setOpen,trigger]);
 useLayoutEffect(()=>()=>{afterExit.current=null;trigger.current?.style.removeProperty('--menu-join-height');trigger.current?.style.removeProperty('--menu-shadow-opacity');generation.current++;cancelAnimationFrame(frame.current);animations.current.forEach(animation=>animation.cancel());animations.current=[];},[]);
 const onEscapeKeyDown=useCallback((event:KeyboardEvent)=>{if(visible.current)return;
  // An outgoing visual is inert immediately. Radix retains its global layer
  // until unmount, so hand this Escape to the newly opened modal explicitly.
  event.preventDefault();const modal=[...document.querySelectorAll<HTMLDialogElement>('dialog[open]:not([inert])')].at(-1);modal?.dispatchEvent(new Event('cancel',{cancelable:true}));
 },[]);
 return {open,present,session,setOpen,closeThen,ref,content,element,onEscapeKeyDown};
}
