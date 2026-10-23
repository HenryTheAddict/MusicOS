import TrackCrossfade from './TrackCrossfade';
import {useEffect,useRef,useMemo} from 'react';
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
 // Both compositions stay mounted: CSS opacity transitions reverse in place.
 useEffect(()=>{root.current?.focus();},[]);
 useEffect(()=>{
  const escape=(e:KeyboardEvent)=>{
   if(e.key!=='Escape'||e.defaultPrevented||document.querySelector('.mini-actions-popover:popover-open,[role=menu][data-state=open]'))return;
   e.preventDefault();mode==='queue'?actions.current.onQueue():actions.current.onMode('normal');
  };
  window.addEventListener('keydown',escape);
  return()=>window.removeEventListener('keydown',escape);
 },[mode]);
 return <section ref={root} tabIndex={-1} className="mini-player" data-layout={mode} aria-label="Mini Player" onMouseDown={e=>dragWindow(e,onWindowError)} onContextMenu={e=>{if(e.defaultPrevented)return;e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} onKeyDown={e=>{if(e.defaultPrevented)return;if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new Event('musicos-open-menu'));}if(e.key==='Escape'&&!document.querySelector('.mini-actions-popover:popover-open')&&!e.defaultPrevented&&!((e.target as HTMLElement).closest('[role=menu]'))){e.preventDefault();mode==='queue'?onQueue():onMode('normal');}}}>
  <div className="mini-player-content">
   {(['tall','queue'] as const).map(view=><div key={view} className={`mini-composition mini-${view}-view`} aria-hidden={mode!==view} inert={mode!==view} data-active={mode===view}>
    <button data-track-visual className={view==='tall'?'mini-player-art':'mini-player-thumb'} onClick={onAlbum} aria-label="Show playing album" disabled={!track}><TrackCrossfade identity={track?.id}><Cover src={cover} trackId={track?.id} title={track?.album??'No music playing'}/></TrackCrossfade></button>
    <div className="mini-player-copy"><TrackCrossfade identity={track?.id}><strong title={track?.title}><OverflowLabel text={track?.title??'Nothing playing yet'}/></strong><button className="mini-player-album" onClick={onAlbum} disabled={!track} aria-label="Show playing album details"><OverflowLabel text={track?`${trackCredits(track).join(' + ')} · ${track.album}`:'Choose a song in Library'}/></button></TrackCrossfade><PlaybackProgress trackId={track?.id} position={player.position} duration={track?.duration??0} showTime transition={player.dj_transition} seek={position=>onAction({type:'seek',position})}/></div>
    {view==='queue'&&<div className="mini-queue-list" role="region" aria-label="Up next" aria-hidden={mode!=='queue'} inert={mode!=='queue'}><header className="mini-queue-heading"><strong>Up next</strong><span>{upcomingCount} {upcomingCount===1?'song':'songs'}</span></header>{queueContent}</div>}
   </div>)}
  </div>
  <div className="mini-player-tools">{track?<FavoriteButton track={track} bpm={bpm} playing={player.is_playing} className="icon-button mini-favorite" label={track.favorited?'Unfavorite song':'Favorite song'} onFavorite={onFavorite}/>:<IconButton name="noheart" label="Favorite song" disabled onClick={onFavorite}/>}<IconButton name={mode==='queue'?'tinyX':'queue'} label={mode==='queue'?'Close Up next':'Open Up next'} pressed={mode==='queue'} onClick={onQueue}/><MiniActions height={320} onResize={onMenuResize} items={[{label:'Return to MusicOS',icon:'CloseMiniplayer',onSelect:()=>onMode('normal')},{label:pinned?'Unpin window':'Pin window',icon:pinned?'unPin':'Pin',onSelect:onPin},{label:mode==='queue'?'Hide Up next':'Up next',icon:'queue',onSelect:onQueue},...items]}/></div>
  <div className="mini-player-transport"><IconButton name="prev" label="Previous track" disabled={!track} onClick={()=>void onAction({type:'previous'})}/><IconButton name={player.is_playing?'pause':'play'} label={player.is_playing?'Pause':'Play'} disabled={!track} onClick={()=>void onAction({type:'toggle'})}/><IconButton name="next" label="Next track" disabled={!track} onClick={()=>void onAction({type:'next'})}/></div>
  <MiniResize onError={onWindowError}/>
 </section>;
}
