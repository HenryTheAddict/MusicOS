import {useEffect,useLayoutEffect,useRef,type CSSProperties} from 'react';
import {AssetIcon} from './AssetIcon';
import {watchHeart} from './heartVisibility';
import type {Track} from './types';
export default function FavoriteButton({track,bpm,playing=false,onFavorite,className='',label}:{track:Track;bpm?:number;playing?:boolean;onFavorite?:(track:Track)=>void;className?:string;label?:string}){
 const button=useRef<HTMLButtonElement>(null),previous=useRef(track.favorited);
 const period=bpm!==undefined&&Number.isFinite(bpm)&&bpm>=20&&bpm<=400?60000/bpm:undefined;
 useEffect(()=>{if(playing&&period!==undefined&&button.current)return watchHeart(button.current);},[playing,period]);
 useLayoutEffect(()=>{
  const element=button.current!;
  if(previous.current===track.favorited)return;
  previous.current=track.favorited;
  if(window.matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  const heart=element.querySelector('.heart-motion')!;
  const feedback=heart.animate(track.favorited?[{transform:'scale(1)'},{transform:'scale(1.1)',offset:.45},{transform:'scale(1)'}]:[{opacity:.85},{opacity:1}],{duration:200,easing:'ease-out'});
  return()=>feedback.cancel();
 },[track.favorited]);
 return <button ref={button} className={`track-favorite ${className}`} style={period===undefined?undefined:{'--heart-period':`${period}ms`} as CSSProperties} aria-label={label??`${track.favorited?'Unfavorite':'Favorite'} ${track.title}`} aria-pressed={track.favorited} onDoubleClick={e=>e.stopPropagation()} onClick={e=>{e.stopPropagation();onFavorite?.(track);}}><span className="heart-beat"><span className="heart-motion"><span className="heart-layer heart-empty" data-active={!track.favorited}><AssetIcon name="noheart"/></span><span className="heart-layer heart-filled" data-active={track.favorited}><AssetIcon name="hearted"/></span></span></span></button>;
}
