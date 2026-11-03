import {hoverHeart,releaseHeart,haptic} from './haptics';
import {useEffect,useLayoutEffect,useRef,type CSSProperties} from 'react';
import {AssetIcon} from './AssetIcon';
import {watchHeart} from './heartVisibility';
import {useViewActive} from './ViewScope';
import type {Track} from './types';
export default function FavoriteButton({track,bpm,playing=false,onFavorite,className='',label}:{track:Track;bpm?:number;playing?:boolean;onFavorite?:(track:Track)=>void;className?:string;label?:string}){
 const active=useViewActive();
 const beating=active&&playing;
 const button=useRef<HTMLButtonElement>(null),previous=useRef({id:track.id,liked:track.favorited});
 useEffect(()=>{const node=button.current;return()=>{if(node)releaseHeart(node);};},[]);
 const period=bpm!==undefined&&Number.isFinite(bpm)&&bpm>=20&&bpm<=400?60000/bpm:undefined;
 useEffect(()=>{if(beating&&period!==undefined&&button.current){const stop=watchHeart(button.current);return stop;}if(button.current)releaseHeart(button.current);},[beating,period]);
 useLayoutEffect(()=>{
  const element=button.current!;
  if(!active){previous.current={id:track.id,liked:track.favorited};return;}
  if(previous.current.id!==track.id){previous.current={id:track.id,liked:track.favorited};return;}
  if(previous.current.liked===track.favorited)return;
  previous.current={id:track.id,liked:track.favorited};
  haptic('favorite',String(track.id));
  if(window.matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  const heart=element.querySelector('.heart-motion')!;
  const feedback=heart.animate(track.favorited?[{transform:'scale(1) rotate(0)'},{transform:'scale(.75) rotate(-12deg)',offset:.16},{transform:'scale(1.4) rotate(8deg)',offset:.42},{transform:'scale(.94) rotate(-3deg)',offset:.72},{transform:'scale(1) rotate(0)'}]:[{transform:'scale(1)'},{transform:'scale(.85)',offset:.35},{transform:'scale(1)'}],{duration:track.favorited?420:180,easing:'cubic-bezier(.2,.7,.2,1)'});
  const celebrations:Animation[]=[];if(track.favorited){const ring=element.querySelector('.heart-celebration>b');if(ring)celebrations.push(ring.animate([{opacity:.65,transform:'scale(.5)'},{opacity:0,transform:'scale(2.1)'}],{duration:380,easing:'ease-out'}));element.querySelectorAll('.heart-celebration>i').forEach((dot,i)=>{const a=i*Math.PI/4;celebrations.push(dot.animate([{opacity:0,transform:'translate(0,0) scale(.5)'},{opacity:1,offset:.2},{opacity:0,transform:`translate(${Math.cos(a)*23}px,${Math.sin(a)*23}px) scale(.4)`}],{duration:400,easing:'ease-out'}));});}
  void Promise.allSettled([feedback,...celebrations].map(a=>a.finished)).then(()=>{feedback.cancel();celebrations.forEach(a=>a.cancel());});
  return()=>{feedback.cancel();celebrations.forEach(a=>a.cancel());};
 },[track.id,track.favorited,active]);
 return <button onPointerEnter={()=>{if(button.current)hoverHeart(button.current);}} onPointerLeave={()=>hoverHeart(null)} ref={button} className={`track-favorite ${className}`} style={period===undefined?undefined:{'--heart-period':`${period}ms`} as CSSProperties} aria-label={label??`${track.favorited?'Unfavorite':'Favorite'} ${track.title}`} data-heart-track={track.id} data-bpm={bpm} data-playing={beating} data-tempo-source={bpm!==undefined&&Number.isFinite(bpm)&&bpm>=20&&bpm<=400?'analyzed':'unknown'} title={bpm!==undefined&&Number.isFinite(bpm)&&bpm>=20&&bpm<=400?`${Math.round(bpm)} BPM${playing?' · Beating with the song':''}`:undefined} aria-pressed={track.favorited} onDoubleClick={e=>e.stopPropagation()} onClick={e=>{e.stopPropagation();onFavorite?.(track);}}><span className="heart-beat"><span className="heart-motion"><span className="heart-layer heart-empty" data-active={!track.favorited}><AssetIcon name="noheart"/></span><span className="heart-layer heart-filled" data-active={track.favorited}><AssetIcon name="hearted"/></span></span></span><span className="heart-celebration" aria-hidden="true"><b/>{[0,1,2,3,4,5,6,7].map(i=><i key={i}/>)}</span></button>;
}
