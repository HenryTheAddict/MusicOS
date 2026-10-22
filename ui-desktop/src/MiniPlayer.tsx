import {useEffect,useLayoutEffect,useRef,useMemo,useState} from 'react';
import FavoriteButton from './FavoriteButton';
import OverflowLabel from './OverflowLabel';
import {trackCredits} from './credits';
import {Cover,IconButton,QueueRows,type MenuItem} from './components';
import MiniActions from './MiniActions';
import MiniResize from './MiniResize';
import {dragWindow} from './windowDrag';
import PlaybackProgress from './PlaybackProgress';
import type {Playback,PlaybackAction} from './types';

export default function MiniPlayer({player,cover,bpm,mode,onMode,onAction,onFavorite,onAlbum,onQueue,onMenuResize,pinned,onPin,onWindowError,items}:{player:Playback;cover?:string|null;bpm?:number;mode:'tall'|'queue';onMode:(mode:'normal'|'tall'|'queue')=>void;onAction:(action:PlaybackAction)=>Promise<void>;onFavorite:()=>void;onAlbum:()=>void;onQueue:()=>void;onMenuResize:(open:boolean)=>Promise<void>;pinned:boolean;onPin:()=>void;onWindowError:(message:string)=>void;items:MenuItem[]}){
 const track=player.track;
 const upcomingCount=Math.max(0,player.queue.length-(player.queue_index===null?0:player.queue_index+1));
 const queueContent=useMemo(()=>{
  const offset=player.queue_index===null?0:player.queue_index+1,upcoming=player.queue.slice(offset);
  return upcoming.length?<QueueRows tracks={upcoming} entryIds={player.queue_entry_ids?.slice(offset)} arranged={player.arranged_entries} pinned={player.pinned_entries} current={null} scale={.625} onSelect={index=>void onAction({type:'select',index:index+offset})} onRemove={index=>void onAction({type:'remove',index:index+offset})} onMove={(from,to)=>void onAction({type:'move',from:from+offset,to:to+offset})}/>:<p className="mini-queue-empty">Nothing up next.<br/>Add music in Library.</p>;
 },[player.queue,player.queue_entry_ids,player.queue_index,player.arranged_entries,player.pinned_entries,onAction]);
 const actions=useRef({onMode,onQueue});actions.current={onMode,onQueue};
 const root=useRef<HTMLElement>(null);
 const [displayedMode,setDisplayedMode]=useState(mode);
 const fade=useRef<Animation[]>([]);
 const lastMode=useRef(mode);
 useLayoutEffect(()=>{
  const content=[...(root.current?.querySelectorAll<HTMLElement>('.mini-player-art,.mini-player-copy,.mini-queue-list')??[])];
  const interrupted=fade.current.some(animation=>animation.playState==='running');
  const opacity=content.map(node=>getComputedStyle(node).opacity);
  fade.current.forEach(animation=>animation.cancel());fade.current=[];
  if(window.matchMedia('(prefers-reduced-motion: reduce)').matches){setDisplayedMode(mode);return;}
  if(mode===displayedMode){
   if(lastMode.current!==mode||interrupted)fade.current=content.flatMap((node,index)=>node.className==='mini-queue-list'&&mode!=='queue'?[]:[node.animate([{opacity:lastMode.current===mode?opacity[index]:0},{opacity:1}],{duration:105,easing:'ease-out'})]);
   lastMode.current=mode;return;
  }
  // Fade the old layout out before reflow. Controls never leave their bottom frame.
  let live=true;
  fade.current=content.map((node,index)=>node.animate([{opacity:opacity[index]},{opacity:0}],{duration:75,easing:'ease-out',fill:'forwards'}));
  void Promise.all(fade.current.map(animation=>animation.finished)).then(()=>{if(live)setDisplayedMode(mode);}).catch(()=>{});
  return()=>{live=false;};
 },[mode,displayedMode]);
 useEffect(()=>()=>fade.current.forEach(animation=>animation.cancel()),[]);
 useEffect(()=>{root.current?.focus();},[]);
 useEffect(()=>{
  const escape=(e:KeyboardEvent)=>{
   if(e.key!=='Escape'||e.defaultPrevented||document.querySelector('.mini-actions-popover:popover-open,[role=menu][data-state=open]'))return;
   e.preventDefault();mode==='queue'?actions.current.onQueue():actions.current.onMode('normal');
  };
  window.addEventListener('keydown',escape);
  return()=>window.removeEventListener('keydown',escape);
 },[mode]);
 return <section ref={root} tabIndex={-1} className="mini-player" data-layout={displayedMode} aria-label="Mini Player" onMouseDown={e=>dragWindow(e,onWindowError)} onContextMenu={e=>{if(e.defaultPrevented)return;e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} onKeyDown={e=>{if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new Event('musicos-open-menu'));}if(e.key==='Escape'&&!document.querySelector('.mini-actions-popover:popover-open')&&!e.defaultPrevented&&!((e.target as HTMLElement).closest('[role=menu]'))){e.preventDefault();mode==='queue'?onQueue():onMode('normal');}}}>
  <button className="mini-player-art" onClick={onAlbum} aria-label="Show playing album" disabled={!track}><Cover src={cover} trackId={track?.id} title={track?.album??'No music playing'}/></button>
  <div className="mini-player-copy"><strong title={track?.title}><OverflowLabel text={track?.title??'Nothing playing yet'}/></strong><button className="mini-player-album" onClick={onAlbum} disabled={!track} aria-label="Show playing album details" title={track?`${track.artist} · ${track.album}`:undefined}><OverflowLabel text={track?`${trackCredits(track).join(' + ')} · ${track.album}`:'Choose a song in Library'}/></button><PlaybackProgress trackId={track?.id} position={player.position} duration={track?.duration??0} showTime transition={player.dj_transition} seek={position=>onAction({type:'seek',position})}/></div>
  <div className="mini-queue-list" role="region" aria-label="Up next" aria-hidden={mode!=='queue'} inert={mode!=='queue'}><header className="mini-queue-heading"><strong>Up next</strong><span>{upcomingCount} {upcomingCount===1?'song':'songs'}</span></header>{queueContent}</div>
  <div className="mini-player-tools">{track?<FavoriteButton track={track} bpm={bpm} playing={player.is_playing} className="icon-button mini-favorite" label={track.favorited?'Unfavorite song':'Favorite song'} onFavorite={onFavorite}/>:<IconButton name="noheart" label="Favorite song" disabled onClick={onFavorite}/>}<IconButton name="queue" label={mode==='queue'?'Close Up next':'Open Up next'} pressed={mode==='queue'} onClick={onQueue}/><MiniActions height={320} onResize={onMenuResize} items={[{label:'Return to MusicOS',icon:'CloseMiniplayer',onSelect:()=>onMode('normal')},{label:pinned?'Unpin window':'Pin window',icon:pinned?'unPin':'Pin',onSelect:onPin},{label:mode==='queue'?'Hide Up next':'Up next',icon:'queue',onSelect:onQueue},...items]}/></div>
  <div className="mini-player-transport"><IconButton name="prev" label="Previous track" disabled={!track} onClick={()=>void onAction({type:'previous'})}/><IconButton name={player.is_playing?'pause':'play'} label={player.is_playing?'Pause':'Play'} disabled={!track} onClick={()=>void onAction({type:'toggle'})}/><IconButton name="next" label="Next track" disabled={!track} onClick={()=>void onAction({type:'next'})}/></div>
  <MiniResize onError={onWindowError}/>
 </section>;
}
