import {useEffect,useState,type RefObject} from 'react';

/** A single idle timeout, independent of playback ticks and lyric updates. */
export function usePlayerChrome(host:RefObject<HTMLDialogElement|null>,open:boolean){
 const [visible,setVisible]=useState(true);
 useEffect(()=>{
  const node=host.current;if(!node||!open)return;
  let timer:ReturnType<typeof setTimeout>|undefined,shown=true,swallowClick=false;
  const reveal=()=>{if(!shown){shown=true;setVisible(true);}clearTimeout(timer);timer=setTimeout(()=>{
   const focused=document.activeElement;
   const keyboardControl=document.documentElement.dataset.inputModality==='keyboard'&&focused instanceof Element&&node.contains(focused)&&!!focused.closest('.expanded-player-header,.lyrics-tools');
   if(node.querySelector('[aria-expanded=true]')||node.querySelector('.expanded-player-header:hover,.lyrics-tools:hover')||keyboardControl)return;
   shown=false;setVisible(false);
  },2200);};
  const press=(event:PointerEvent)=>{swallowClick=!shown&&event.pointerType==='touch';if(swallowClick){event.preventDefault();event.stopPropagation();}reveal();};
  const click=(event:MouseEvent)=>{if(swallowClick){swallowClick=false;event.preventDefault();event.stopImmediatePropagation();}};
  const visibility=()=>{if(document.hidden)clearTimeout(timer);else reveal();};
  node.addEventListener('pointermove',reveal);node.addEventListener('pointerleave',reveal);node.addEventListener('pointerdown',press,true);node.addEventListener('keydown',reveal,true);node.addEventListener('focusin',reveal);node.addEventListener('focusout',reveal);
  node.addEventListener('click',click,true);
  document.addEventListener('visibilitychange',visibility);
  const menu=new MutationObserver(reveal);menu.observe(node,{subtree:true,attributes:true,attributeFilter:['aria-expanded']});
  setVisible(true);reveal();
  return()=>{clearTimeout(timer);menu.disconnect();document.removeEventListener('visibilitychange',visibility);node.removeEventListener('pointermove',reveal);node.removeEventListener('pointerleave',reveal);node.removeEventListener('pointerdown',press,true);node.removeEventListener('click',click,true);node.removeEventListener('keydown',reveal,true);node.removeEventListener('focusin',reveal);node.removeEventListener('focusout',reveal);};
 },[host,open]);
 return visible?'visible':'hidden';
}
