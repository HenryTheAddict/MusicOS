import {useLayoutEffect,useRef} from 'react';
import {SketchIcon} from './SketchIcon';
import type {Track} from './types';
export default function FavoriteButton({track,onFavorite}:{track:Track;onFavorite?:(track:Track)=>void}){
 const button=useRef<HTMLButtonElement>(null),previous=useRef(track.favorited);
 useLayoutEffect(()=>{
  const element=button.current!;
  if(previous.current===track.favorited)return;
  previous.current=track.favorited;
  element.getAnimations({subtree:true}).forEach(animation=>animation.cancel());
  if(window.matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  const heart=element.querySelector('.heart-motion')!;
  heart.animate(track.favorited?[{transform:'scale(1)'},{transform:'scale(1.1)',offset:.45},{transform:'scale(1)'}]:[{opacity:.85},{opacity:1}],{duration:200,easing:'ease-out'});
  return()=>element.getAnimations({subtree:true}).forEach(animation=>animation.cancel());
 },[track.favorited]);
 return <button ref={button} className="track-favorite" aria-label={`${track.favorited?'Unfavorite':'Favorite'} ${track.title}`} aria-pressed={track.favorited} onDoubleClick={e=>e.stopPropagation()} onClick={e=>{e.stopPropagation();onFavorite?.(track);}}><span className="heart-motion"><span className="heart-layer heart-empty" data-active={!track.favorited}><SketchIcon name="noheart"/></span><span className="heart-layer heart-filled" data-active={track.favorited}><SketchIcon name="hearted"/></span></span></button>;
}
