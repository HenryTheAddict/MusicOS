import {useLayoutEffect, useRef, useState} from 'react';

const easing = 'cubic-bezier(.2,.72,.26,1)';
export const playerEntrance = 'phone-player-entrance';

export function allowsPhoneMotion(enabled: boolean) {
 return enabled && !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function displayedPlayerOffset(sheet: HTMLElement) {
 const offset = Number.parseFloat(getComputedStyle(sheet).translate.split(' ')[1] ?? '0');
 return Number.isFinite(offset) ? Math.max(0, offset) : 0;
}

/** The button and gesture share one sheet transition. A drag can take over the entrance. */
export function usePhonePlayerSheet(motionEnabled: boolean) {
 const [expanded,setExpanded] = useState(false);
 const [closing,setClosing] = useState(false);
 const sheet = useRef<HTMLElement>(null);
 const animation = useRef<Animation|null>(null);
 const returnFocus = useRef<HTMLElement|null>(null);
 const closingNow = useRef(false);

 function open(opener?: HTMLElement) {
  if (closingNow.current || expanded) return;
  returnFocus.current = opener ?? document.querySelector<HTMLElement>('.mini-identify');
  setExpanded(true);
 }

 function finish() {
  closingNow.current = false;
  setExpanded(false);
  setClosing(false);
 }

 function close(pulledOffset?: number) {
  const node = sheet.current;
  if (!node || closingNow.current) return;
  closingNow.current = true;
  if (!allowsPhoneMotion(motionEnabled) || typeof node.animate !== 'function') {
   finish();
   return;
  }
  node.setAttribute('data-swipe-closing','true');
  const from = pulledOffset ?? displayedPlayerOffset(node);
  const opacity = getComputedStyle(node).opacity;
  animation.current?.cancel();
  setClosing(true);
  const transition = node.animate(
   [{translate:`0 ${from}px`,opacity},{translate:`0 ${node.offsetHeight}px`,opacity:.98}],
   {duration:180,easing,fill:'forwards'},
  );
  transition.id = 'phone-player-exit';
  animation.current = transition;
  void transition.finished.then(()=>{
   if (animation.current === transition) finish();
  }).catch(()=>{});
 }

 useLayoutEffect(()=>{
  if (!expanded) {
   const target = returnFocus.current;
   returnFocus.current = null;
   if (target?.isConnected) target.focus({preventScroll:true});
   return;
  }
  const node = sheet.current;
  if (!node) return;
  if (allowsPhoneMotion(motionEnabled) && typeof node.animate === 'function') {
   const transition = node.animate(
    [{translate:`0 ${Math.min(96,node.offsetHeight*.14)}px`,opacity:.96},{translate:'0 0',opacity:1}],
    {duration:180,easing},
   );
   transition.id = playerEntrance;
   animation.current = transition;
   void transition.finished.then(()=>{
    if (animation.current === transition) animation.current = null;
   }).catch(()=>{});
  }
  const media = window.matchMedia('(prefers-reduced-motion: reduce)');
  const settle = ()=>{
   if (!media.matches) return;
   animation.current?.cancel();
   animation.current = null;
   if (closingNow.current) finish();
  };
  const hidden = ()=>{
   if (!document.hidden) return;
   animation.current?.cancel();
   animation.current = null;
   if (closingNow.current) finish();
  };
  media.addEventListener('change',settle);
  document.addEventListener('visibilitychange',hidden);
  return ()=>{
   media.removeEventListener('change',settle);
   document.removeEventListener('visibilitychange',hidden);
   animation.current?.cancel();
   animation.current = null;
  };
 },[expanded]);

 return {expanded,closing,sheet,open,close};
}
