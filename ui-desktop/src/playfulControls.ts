import {useEffect} from 'react';
import {animateOwned} from './presentationMotion';

/** React only to an intentional action. Hit boxes, labels and artwork stay still. */
export function usePlayfulControls(){
 useEffect(()=>{
  const active=new Map<HTMLElement,()=>void>();
  const clear=()=>{active.forEach(cancel=>cancel());active.clear();};
  const click=(event:MouseEvent)=>{
   const button=event.target instanceof Element?event.target.closest<HTMLButtonElement>('button[data-playful]'):null;
   if(!button||button.disabled||button.closest('[inert],[hidden]')||matchMedia('(prefers-reduced-motion: reduce)').matches)return;
   const glyph=button.querySelector<HTMLElement>(':scope>.icon');if(!glyph)return;
   const cancel=animateOwned(glyph,'playful-icon-response',[{transform:'rotate(0deg) translateY(0)'},{transform:'rotate(-9deg) translateY(-1px)',offset:.32},{transform:'rotate(4deg) translateY(0)',offset:.68},{transform:'rotate(0deg) translateY(0)'}],180);
   active.set(glyph,cancel);const animation=glyph.getAnimations().find(a=>a.id==='playful-icon-response');
   void animation?.finished.then(()=>{if(active.get(glyph)===cancel)active.delete(glyph);}).catch(()=>{if(active.get(glyph)===cancel)active.delete(glyph);});
  };
  document.addEventListener('click',click);document.addEventListener('scroll',clear,true);document.addEventListener('visibilitychange',clear);window.addEventListener('resize',clear);
  return()=>{clear();document.removeEventListener('click',click);document.removeEventListener('scroll',clear,true);document.removeEventListener('visibilitychange',clear);window.removeEventListener('resize',clear);};
 },[]);
}
