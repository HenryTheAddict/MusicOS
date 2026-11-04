import {useEffect,useState,type RefObject} from 'react';

/** A single idle timeout, independent of playback ticks and lyric updates. */
export function usePlayerChrome(host:RefObject<HTMLDialogElement|null>,open:boolean){
 const [visible,setVisible]=useState(false);
 useEffect(()=>{
  const node=host.current;if(!node||!open)return;
  let timer:ReturnType<typeof setTimeout>|undefined,shown=false,swallowClick=false;
  const reveal=()=>{if(!shown){shown=true;setVisible(true);}clearTimeout(timer);timer=setTimeout(()=>{
   const focused=document.activeElement;
   const keyboardControl=(document.documentElement.dataset.inputModality==='keyboard'||document.documentElement.dataset.controllerInput==='true')&&focused instanceof Element&&node.contains(focused)&&!!focused.closest('.expanded-player-header,.expanded-player-controls');
   if(node.querySelector('input:active,[data-seeking=true],[role=menu]:not([inert])')||node.querySelector('[aria-expanded=true]')||keyboardControl)return;
   shown=false;setVisible(false);
  },2200);};
  let lastPoint:[number,number]|undefined;
  const move=(event:PointerEvent)=>{if(lastPoint&&lastPoint[0]===event.clientX&&lastPoint[1]===event.clientY)return;lastPoint=[event.clientX,event.clientY];reveal();};
  const press=(event:PointerEvent)=>{swallowClick=!shown&&event.pointerType==='touch';if(swallowClick){event.preventDefault();event.stopPropagation();}reveal();};
  const click=(event:MouseEvent)=>{if(swallowClick){swallowClick=false;event.preventDefault();event.stopImmediatePropagation();}};
  const blur=()=>{if(shown)reveal();};
  const focus=(event:FocusEvent)=>{if(event.target!==node&&(shown||(document.documentElement.dataset.inputModality==='keyboard'||document.documentElement.dataset.controllerInput==='true')))reveal();};
  const visibility=()=>{if(document.hidden)clearTimeout(timer);else reveal();};
  node.addEventListener('pointermove',move);node.addEventListener('pointerleave',move);node.addEventListener('pointerdown',press,true);node.addEventListener('pointerup',reveal,true);node.addEventListener('keydown',reveal,true);node.addEventListener('focusin',focus);node.addEventListener('focusout',blur);
  node.addEventListener('click',click,true);
  window.addEventListener('musicos-player-interaction',reveal);
  document.addEventListener('visibilitychange',visibility);
  const menu=new MutationObserver(records=>{if(records.some(record=>record.oldValue!==(record.target as Element).getAttribute(record.attributeName!)))reveal();});menu.observe(node,{subtree:true,attributes:true,attributeOldValue:true,attributeFilter:['aria-expanded','data-seeking']});
  setVisible(false);
  return()=>{clearTimeout(timer);menu.disconnect();window.removeEventListener('musicos-player-interaction',reveal);document.removeEventListener('visibilitychange',visibility);node.removeEventListener('pointermove',move);node.removeEventListener('pointerleave',move);node.removeEventListener('pointerdown',press,true);node.removeEventListener('pointerup',reveal,true);node.removeEventListener('click',click,true);node.removeEventListener('keydown',reveal,true);node.removeEventListener('focusin',focus);node.removeEventListener('focusout',blur);};
 },[host,open]);
 return visible?'visible':'hidden';
}
