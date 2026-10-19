import {useId,useLayoutEffect,useRef,useState} from 'react';
import {usePortraitArtwork} from './portraitArtwork';
import {artwork} from './bridge';
import {PortraitLoading} from './components';
import type {Artist} from './types';
export default function ArtistCredit({artist,primary,loading,onOpen}:{artist:Artist;primary:boolean;loading:boolean;onOpen:()=>void}){
 const portrait=usePortraitArtwork(artist.portrait);
 const id=useId(),button=useRef<HTMLButtonElement>(null),tip=useRef<HTMLSpanElement>(null),point=useRef<{x:number;y:number}|null>(null),frame=useRef(0);
 const [open,setOpen]=useState(false);
 const position=()=>{const anchor=button.current,panel=tip.current;if(!anchor||!panel)return;const rect=anchor.getBoundingClientRect(),box=panel.getBoundingClientRect(),p=point.current;const x=p?p.x+14:rect.left,y=p?p.y+18:rect.bottom+8;panel.style.left=`${Math.max(8,Math.min(x,innerWidth-box.width-8))}px`;panel.style.top=`${Math.max(8,Math.min(y,innerHeight-box.height-8))}px`;};
 useLayoutEffect(()=>{const panel=tip.current;if(!panel||primary)return;if(!open){panel.hidePopover();return;}panel.dataset.following='false';panel.style.visibility='hidden';panel.showPopover();position();void panel.offsetWidth;panel.style.visibility='';panel.dataset.following='true';const close=()=>setOpen(false);const escape=(e:KeyboardEvent)=>{if(e.key==='Escape')close();};window.addEventListener('scroll',close,true);window.addEventListener('resize',close);window.addEventListener('blur',close);window.addEventListener('keydown',escape);return()=>{cancelAnimationFrame(frame.current);panel.hidePopover();window.removeEventListener('scroll',close,true);window.removeEventListener('resize',close);window.removeEventListener('blur',close);window.removeEventListener('keydown',escape);};},[open,primary]);
 return <button ref={button} className={`artist-link ${primary?'album-primary-artist':'album-collaborator'}`} aria-label={`View artist ${artist.name}`} aria-describedby={!primary&&open?id:undefined} onClick={()=>{setOpen(false);onOpen();}} onPointerEnter={e=>{point.current={x:e.clientX,y:e.clientY};setOpen(true);}} onPointerMove={e=>{point.current={x:e.clientX,y:e.clientY};cancelAnimationFrame(frame.current);frame.current=requestAnimationFrame(position);}} onPointerLeave={()=>setOpen(false)} onFocus={e=>{if(e.currentTarget.matches(':focus-visible')){point.current=null;setOpen(true);}}} onBlur={()=>setOpen(false)}>
 <span className="artist-avatar" aria-hidden="true">{artist.portrait?<img src={artwork(portrait?.display)} alt="" draggable={false}/>:artist.name.slice(0,1)}{!artist.portrait&&loading&&<PortraitLoading/>}</span>
 {primary?<span className="artist-credit-name">{artist.name}</span>:<span ref={tip} id={id} popover="manual" role="tooltip" className="artist-credit-name artist-name-popover">{artist.name}</span>}
 </button>;
}
