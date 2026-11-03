import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import type {Track} from './types';
import {Cover} from './components';
import {animateOwned} from './presentationMotion';

/** Acknowledged handoff feedback, including the artwork belonging to that song. */
export default function SongChangeNotice({track,cover,entryId,active}:{track:Track|null;cover?:string|null;entryId?:number;active:boolean}){
 const identity=`${track?.id??''}:${entryId??''}`,last=useRef(identity),host=useRef<HTMLDivElement>(null),direction=useRef(1);
 const [notice,setNotice]=useState<{track:Track;cover?:string|null;identity:string}|null>(null),[visible,setVisible]=useState(false);
 useEffect(()=>{const intent=(e:Event)=>{direction.current=(e as CustomEvent).detail==='previous'?-1:1;};window.addEventListener('musicos-transport-intent',intent);return()=>window.removeEventListener('musicos-transport-intent',intent);},[]);
 useEffect(()=>{
  const changed=last.current!==identity&&!!last.current.split(':')[0];last.current=identity;
  if(!active||!track||!changed){setVisible(false);setNotice(null);return;}
  setNotice({track,cover:cover??track.cover_art_path,identity});setVisible(true);
  const hide=setTimeout(()=>setVisible(false),2200),clear=setTimeout(()=>setNotice(null),2380);
  return()=>{clearTimeout(hide);clearTimeout(clear);};
 },[identity,active]);
 useLayoutEffect(()=>{const node=host.current?.firstElementChild as HTMLElement|null;if(node&&notice)return animateOwned(node,'song-notice-handoff',[{opacity:.2,transform:`translateX(${direction.current*12}px)`},{opacity:1,transform:'none'}],220);},[notice?.identity]);
 return <div ref={host} className="song-change-notice" data-visible={visible&&active||undefined} role="status" aria-live="polite">{notice&&<div className="song-change-content"><Cover songArtwork src={notice.cover} trackId={notice.track.id} title={notice.track.album}/><span><small>Now playing</small><strong>{notice.track.title}</strong><small>{notice.track.artist}</small></span></div>}</div>;
}
