import {useEffect,useRef,useState} from 'react';
/** One short ring on acknowledged playback start; never a perpetual background loop. */
export default function PlaybackRipple({playing,active=true}:{playing:boolean;active?:boolean}){
 const previous=useRef(playing),enabled=useRef(active),[pulse,setPulse]=useState(0),[visible,setVisible]=useState(false);
 useEffect(()=>{
  const started=playing&&!previous.current,wasActive=enabled.current;previous.current=playing;enabled.current=active;
  if(!wasActive||!active||!started||matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  setPulse(n=>n+1);setVisible(true);
 },[playing,active]);
 useEffect(()=>{if(!active||!playing)setVisible(false);},[playing,active]);
 if(!visible)return null;
 return <div key={pulse} className="playback-ripple" aria-hidden="true" onAnimationEnd={e=>{if(e.target===e.currentTarget)setVisible(false);}}><i/><i/></div>;
}
