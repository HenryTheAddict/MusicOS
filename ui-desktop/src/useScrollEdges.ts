import {useLayoutEffect,type RefObject} from 'react';
/** Edge fades describe actual overflow, rather than masking controls or short lists. */
export function useScrollEdges(ref:RefObject<HTMLElement|null>,active:boolean,content:unknown){
 useLayoutEffect(()=>{const el=ref.current;if(!active||!el)return;let frame=0;
  const update=()=>{frame=0;el.dataset.scrollTop=String(el.scrollTop>1);el.dataset.scrollBottom=String(el.scrollHeight-el.clientHeight-el.scrollTop>1);};
  const measure=()=>{if(!frame)frame=requestAnimationFrame(update);};
  const resize=new ResizeObserver(measure);resize.observe(el);if(el.firstElementChild)resize.observe(el.firstElementChild);
  el.addEventListener('scroll',measure,{passive:true});update();
  return()=>{resize.disconnect();cancelAnimationFrame(frame);el.removeEventListener('scroll',measure);delete el.dataset.scrollTop;delete el.dataset.scrollBottom;};
 },[ref,active,content]);
}
