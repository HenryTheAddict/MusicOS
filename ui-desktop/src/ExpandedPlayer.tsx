import {useEffect,useLayoutEffect,useRef,type CSSProperties} from 'react';
import {Cover,Icon,IconButton,Overflow,type MenuItem} from './components';
import AudioPanel from './AudioPanel';
import FavoriteButton from './FavoriteButton';
import PlaybackProgress from './PlaybackProgress';
import VolumeControl from './VolumeControl';
import {KeepView} from './ViewScope';
import {animateOwned} from './presentationMotion';
import {native} from './bridge';
import {dragWindow} from './windowDrag';
import type {Playback,PlaybackAction} from './types';
export type PlayerView='playing'|'queue';
export default function ExpandedPlayer({layout="compact",onWindowError=()=>{},open,view,onView,onClose,player,cover,bpm,scale,theme,onAction,onFavorite,items,origin,returnFocus}:{layout?:"wide"|"compact";onWindowError?:(message:string)=>void;returnFocus:HTMLElement|null;open:boolean;view:PlayerView;onView:(view:PlayerView)=>void;onClose:()=>void;player:Playback;cover:string|null;bpm?:number;scale:number;theme:CSSProperties;onAction:(action:PlaybackAction)=>Promise<void>;onFavorite:()=>void;items:MenuItem[];origin:DOMRect|null}){
 const dialog=useRef<HTMLDialogElement>(null),art=useRef<HTMLDivElement>(null),opener=useRef(returnFocus??document.activeElement as HTMLElement|null);
 const menuEscape=useRef(false),visible=useRef(open);visible.current=open;
 // Handle the outer Escape before a retained, inert Radix layer can consume it.
 // The first Escape still belongs entirely to the active nested menu.
 useEffect(()=>{const escape=(event:KeyboardEvent)=>{if(event.key!=='Escape'||event.defaultPrevented||!visible.current)return;menuEscape.current=!!document.querySelector('[role=menu]:not([inert])');if(menuEscape.current)return;if([...document.querySelectorAll('dialog[open]')].at(-1)!==dialog.current)return;event.preventDefault();event.stopImmediatePropagation();dismiss.current();};window.addEventListener('keydown',escape,true);return()=>window.removeEventListener('keydown',escape,true);},[]);
 const dismiss=useRef(onClose);dismiss.current=onClose;
 useEffect(()=>{const node=dialog.current!;node.showModal();return()=>{node.close();if(opener.current?.isConnected&&!document.querySelector('dialog[open]'))opener.current.focus({preventScroll:true});};},[]);
 useLayoutEffect(()=>{const node=dialog.current;if(!node)return;return animateOwned(node,'expanded-player-presence',open?[{opacity:0,transform:'translateY(4px)'},{opacity:1,transform:'none'}]:[{opacity:1,transform:'none'},{opacity:0,transform:'translateY(4px)'}],open?180:140);},[open]);
 useLayoutEffect(()=>{
  const target=art.current,host=dialog.current;if(!open||view!=='playing'||!origin||!target||!host||matchMedia('(prefers-reduced-motion: reduce)').matches)return;
  let frame=0,image:HTMLImageElement|null=null,cancel:undefined|(()=>void);
  const release=()=>{cancelAnimationFrame(frame);cancel?.();image?.remove();target.style.removeProperty('visibility');};
  frame=requestAnimationFrame(()=>{const source=target.querySelector('img');if(!source?.complete)return;const end=target.getBoundingClientRect();if(!end.width)return;image=source.cloneNode(true) as HTMLImageElement;image.alt='';image.className='expanded-player-transfer';image.setAttribute('aria-hidden','true');Object.assign(image.style,{left:`${end.left}px`,top:`${end.top}px`,width:`${end.width}px`,height:`${end.height}px`});host.append(image);target.style.visibility='hidden';cancel=animateOwned(image,'expanded-player-artwork',[{transform:`translate(${origin.left-end.left}px,${origin.top-end.top}px) scale(${origin.width/end.width})`},{transform:'none'}],320);void Promise.allSettled(image.getAnimations().map(a=>a.finished)).then(release);});
  window.addEventListener('resize',release);window.addEventListener('scroll',release,true);return()=>{window.removeEventListener('resize',release);window.removeEventListener('scroll',release,true);release();};
 },[open]);
 const track=player.track;
 return <dialog ref={dialog} style={theme} className="expanded-player" data-layout={layout} data-view={view} data-native-titlebar={native&&/Mac/.test(navigator.platform)||undefined} aria-label="Music player" inert={!open} aria-hidden={!open||undefined} onCancel={event=>{event.preventDefault();if(!menuEscape.current&&!document.querySelector('[role=menu]:not([inert])'))dismiss.current();}} onKeyDown={event=>{const target=event.target as Element;if(event.code==='Space'&&!event.defaultPrevented&&!event.metaKey&&!event.ctrlKey&&!event.altKey&&!target.closest('input,textarea,select,[role=menu]')&&(!target.closest('button')||document.documentElement.dataset.inputModality==='pointer')){event.preventDefault();event.stopPropagation();if(!event.repeat&&track)void onAction({type:'toggle'});}}}>
  <div className="expanded-player-windowbar" aria-hidden="true" onMouseDown={event=>dragWindow(event,onWindowError)}/>
  <header className="expanded-player-header"><div className="player-view-tabs" role="group" aria-label="Player view"><button aria-pressed={view==='playing'} onClick={()=>onView('playing')}>Now playing</button><button aria-pressed={view==='queue'} onClick={()=>onView('queue')}>Up next</button></div><div className="expanded-header-actions"><Overflow items={items}/><button className="expanded-player-close" aria-label="Close player" onClick={onClose} autoFocus><Icon name="controlClose"/></button></div></header>
  <div className="expanded-player-content">
  <KeepView className="expanded-playing-view" active={view==='playing'}><div className="expanded-player-art" ref={art}><Cover songArtwork src={cover} trackId={track?.id} title={track?.album??'No music playing'}/></div><div className="expanded-player-copy"><strong>{track?.title??'Nothing playing yet'}</strong><span>{track?`${track.artist} · ${track.album}`:'Choose music in Library'}</span></div></KeepView>
  <div className="expanded-player-favorite" hidden={view!=='playing'}>{track?<FavoriteButton track={track} bpm={bpm} playing={open&&view==='playing'&&player.is_playing} onFavorite={onFavorite} label="Favorite playing song"/>:<IconButton name="noheart" label="Favorite playing song" disabled onClick={()=>{}}/>}</div>
  <KeepView className="expanded-queue-view" active={view==='queue'}><AudioPanel player={player} scale={scale} theme={theme} presentation="embedded" onAction={onAction} onClose={onClose} onSettings={()=>{}}/></KeepView>
  <div className="expanded-player-controls"><PlaybackProgress trackId={track?.id} position={player.position} duration={track?.duration??0} live={open} seek={async position=>{await onAction({type:'seek',position});}}/>
   <div className="expanded-transport"><IconButton name={player.shuffle?'ShuffleON':'ShuffleOFF'} label="Shuffle" pressed={player.shuffle} disabled={!track} onClick={()=>void onAction({type:'shuffle'})}/><IconButton name="prev" label="Previous track" disabled={!track} onClick={()=>void onAction({type:'previous'})}/><span className="expanded-primary-play"><IconButton name={player.is_playing?'pause':'play'} label={player.is_playing?'Pause':'Play'} disabled={!track} onClick={()=>void onAction({type:'toggle'})}/></span><IconButton name="next" label="Next track" disabled={!track} onClick={()=>void onAction({type:'next'})}/><IconButton name={player.repeat==='one'?'RepeatONE':player.repeat==='all'?'RepeatON':'RepeatOFF'} label="Repeat" disabled={!track} onClick={()=>void onAction({type:'repeat'})}/></div>
   <div className="expanded-player-options"><VolumeControl value={player.volume} onChange={volume=>onAction({type:'volume',volume})}/></div>
  </div>
  </div>
 </dialog>;
}
