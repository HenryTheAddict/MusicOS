import {artwork} from './bridge';
import LyricsView from './LyricsView';
import TrackCrossfade from './TrackCrossfade';
import {useEffect,useLayoutEffect,useRef,useMemo,useState} from 'react';
import {animateOwned} from './presentationMotion';
import FavoriteButton from './FavoriteButton';
import OverflowLabel from './OverflowLabel';
import {trackCredits} from './credits';
import {Cover,Icon,IconButton,QueueRows,QueueTrackCopy,type MenuItem} from './components';
import MiniActions from './MiniActions';
import MiniResize from './MiniResize';
import {dragWindow} from './windowDrag';
import PlaybackProgress from './PlaybackProgress';
import type {Playback,PlaybackAction} from './types';

export default function MiniPlayer({preview=false,density="standard",player,cover,bpm,mode,onMode,onAction,onFavorite,onAlbum,onQueue,onMenuResize,pinned,onPin,onWindowError,items}:{preview?:boolean;density?:"standard"|"compact"|"wide";player:Playback;cover?:string|null;bpm?:number;mode:'tall'|'queue';onMode:(mode:'normal'|'tall'|'queue')=>void;onAction:(action:PlaybackAction)=>Promise<void>;onFavorite:()=>void;onAlbum:()=>void;onQueue:()=>void;onMenuResize:(open:boolean)=>Promise<void>;pinned:boolean;onPin:()=>void;onWindowError:(message:string)=>void;items:MenuItem[]}){
 const [lyrics,setLyrics]=useState(false);const view=lyrics?'lyrics':mode;
 const track=player.track;
 const upcomingCount=Math.max(0,player.queue.length-(player.queue_index===null?0:player.queue_index+1));
 const queueContent=useMemo(()=>{
  const offset=player.queue_index===null?0:player.queue_index+1,upcoming=player.queue.slice(offset);
  return upcoming.length?<QueueRows tracks={upcoming} entryIds={player.queue_entry_ids?.slice(offset)} arranged={player.arranged_entries} pinned={player.pinned_entries} current={null} scale={.625} firstLabel={density!=="standard"?"Up next":undefined} onSelect={index=>void onAction({type:'select',index:index+offset})} onRemove={index=>void onAction({type:'remove',index:index+offset})} onMove={(from,to)=>void onAction({type:'move',from:from+offset,to:to+offset})}/>:<p className="mini-queue-empty">Nothing up next.<span>Add music in Library.</span></p>;
 },[player.queue,player.queue_entry_ids,player.queue_index,player.arranged_entries,player.pinned_entries,onAction,density]);
 const actions=useRef({onMode,onQueue});actions.current={onMode,onQueue};
 const root=useRef<HTMLElement>(null);
 useLayoutEffect(()=>{if(!preview&&root.current)return animateOwned(root.current,'mini-window-enter',[{opacity:0,transform:'translateY(4px)'},{opacity:1,transform:'none'}],180);},[preview]);
 // Both compositions stay mounted: CSS opacity transitions reverse in place.
 useEffect(()=>{if(!preview)root.current?.focus();},[preview]);
 useEffect(()=>{
  if(preview)return;
  const escape=(e:KeyboardEvent)=>{
   if(e.key!=='Escape'||e.defaultPrevented||document.querySelector('.mini-actions-popover:popover-open,[role=menu][data-state=open]'))return;
   e.preventDefault();lyrics?setLyrics(false):mode==='queue'?actions.current.onQueue():actions.current.onMode('normal');
  };
  window.addEventListener('keydown',escape);
  return()=>window.removeEventListener('keydown',escape);
 },[mode,preview,lyrics]);
 return <section ref={root} tabIndex={-1} className="mini-player" data-layout={mode} data-view={view} data-density={density} inert={preview} aria-label="Mini Player" onMouseDown={preview?undefined:e=>dragWindow(e,onWindowError)} onContextMenu={e=>{if(e.defaultPrevented)return;e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} onKeyDown={e=>{if(e.defaultPrevented)return;if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-label="Mini Player actions"]')?.dispatchEvent(new Event('musicos-open-menu'));}if(e.key==='Escape'&&!document.querySelector('.mini-actions-popover:popover-open')&&!e.defaultPrevented&&!((e.target as HTMLElement).closest('[role=menu]'))){e.preventDefault();lyrics?setLyrics(false):mode==='queue'?onQueue():onMode('normal');}}}>
  <div className="mini-view-tabs" role="group" aria-label="Mini Player view">{(['tall','lyrics','queue'] as const).map(value=><button key={value} aria-pressed={view===value} onClick={()=>{if(value==='lyrics')setLyrics(true);else{setLyrics(false);if(mode!==value)onQueue();}}}>{value==='tall'?'Artwork':value==='lyrics'?'Lyrics':'Up next'}</button>)}</div>
  <div className="mini-player-content">{lyrics&&<div className="mini-lyrics-view"><strong>{track?.title??'Nothing playing'}</strong>{preview?<p>Lyrics preview</p>:<LyricsView renderMenu={items=><MiniActions contextOnly height={density==='standard'?320:120} onResize={onMenuResize} items={items}/>} variant="mini" track={track??undefined} player={player} active={!preview} onSeek={position=>onAction({type:'seek',position})} onPlay={async()=>{}}/>}</div>}
   {(['tall','queue'] as const).map(view=><div key={view} className={`mini-composition mini-${view}-view`} aria-hidden={lyrics||mode!==view} inert={lyrics||mode!==view} data-active={!lyrics&&mode===view}>
    <button data-track-visual className={view==='tall'?'mini-player-art':'mini-player-thumb'} onClick={onAlbum} aria-label="Show playing album" disabled={!track}><TrackCrossfade identity={track?.id}><>{preview?(cover?<img className="cover" src={artwork(cover)} alt=""/>:<span className="cover placeholder"><Icon name="noCOVER"/></span>):<Cover src={cover} trackId={track?.id} title={track?.album??'No music playing'}/>}</></TrackCrossfade></button>
    <div className="mini-player-copy"><TrackCrossfade identity={track?.id}><strong title={track?.title}><OverflowLabel text={track?.title??'Nothing playing yet'}/></strong><button className="mini-player-album" onClick={onAlbum} disabled={!track} aria-label="Show playing album details"><OverflowLabel text={track?`${trackCredits(track).join(' + ')} · ${track.album}`:'Choose a song in Library'}/></button></TrackCrossfade><PlaybackProgress revealTime live={!preview&&!lyrics&&mode===view} trackId={track?.id} position={player.position} duration={track?.duration??0} showTime transition={player.dj_transition} seek={position=>onAction({type:'seek',position})}/></div>
    {view==='queue'&&<div className="mini-queue-list" role="region" aria-label="Up next" aria-hidden={mode!=='queue'} inert={mode!=='queue'}><header className="mini-queue-heading"><strong>Up next</strong><span>{upcomingCount} {upcomingCount===1?'song':'songs'}</span></header>{preview?<div className="mini-preview-queue">{player.queue.length?player.queue.map((song,index)=><div className="queue-row" key={player.queue_entry_ids?.[index]??`${song.id}:${index}`}><span className="queue-track">{song.cover_art_path||song.album===track?.album&&cover?<img className="cover" src={artwork(song.cover_art_path??cover)} alt=""/>:<span className="cover placeholder"><Icon name="noCOVER"/></span>}<QueueTrackCopy track={song} leadingLabel={index===0&&density!=="standard"?"Up next":undefined}/></span><span className="queue-remove">×</span></div>):<p className="mini-queue-empty">Nothing up next.<span>Add music in Library.</span></p>}</div>:queueContent}</div>}
   </div>)}
  </div>
  <div className="mini-player-footer"><div className="mini-player-tools">{track?<FavoriteButton track={track} bpm={bpm} playing={player.is_playing} className="icon-button mini-favorite" label={track.favorited?'Unfavorite song':'Favorite song'} onFavorite={onFavorite}/>:<IconButton name="noheart" label="Favorite song" disabled onClick={onFavorite}/>}<IconButton name="queue" label={mode==='queue'?'Close Up next':'Open Up next'} pressed={mode==='queue'} onClick={()=>{setLyrics(false);onQueue();}}/>{preview?<IconButton name="3dots(incircle)" label="Mini Player actions" onClick={()=>{}}/>:<MiniActions height={density==='wide'?120:density==='compact'?120:320} onResize={onMenuResize} items={[{label:'Return to musicOS',icon:'CloseMiniplayer',onSelect:()=>onMode('normal')},{label:pinned?'Unpin window':'Pin window',icon:pinned?'unPin':'Pin',onSelect:onPin},{label:mode==='queue'?'Hide Up next':'Up next',icon:'queue',onSelect:onQueue},...items]}/>}</div>
  <div className="mini-player-transport"><IconButton name="prev" label="Previous track" disabled={!track} onClick={()=>void onAction({type:'previous'})}/><IconButton name={player.is_playing?'pause':'play'} label={player.is_playing?'Pause':'Play'} disabled={!track} onClick={()=>void onAction({type:'toggle'})}/><IconButton name="next" label="Next track" disabled={!track} onClick={()=>void onAction({type:'next'})}/></div>
  </div>
  {!preview&&<MiniResize density={density} onError={onWindowError}/>}
 </section>;
}
