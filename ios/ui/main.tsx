import {songOrder,orderedAlbums,artistTrackOrder,artistOrder} from './libraryOrder';
import PlaylistEditor from './PlaylistEditor';
import ComputerSheet from './ComputerSheet';
import QueueEditor,{AddQueueSongs} from './QueueEditor';
import {useSwipe,SwipeSong,feedback} from './touch';
import {usePhonePlayerSheet,allowsPhoneMotion} from './playerMotion';
import {coverPalette} from './artworkPalette';
import {useScrollEdges} from '../../ui-desktop/src/useScrollEdges';
import React,{useCallback,useEffect,useLayoutEffect,useMemo,useRef,useState,type CSSProperties} from 'react';
import {createRoot} from 'react-dom/client';
import {addPluginListener} from '@tauri-apps/api/core';
import {AssetIcon as Icon} from '../../ui-desktop/src/AssetIcon';
import {collectionCover} from '../../ui-desktop/src/CollectionDesign';
import {animateOwned} from '../../ui-desktop/src/presentationMotion';
import LyricsView from '../../ui-desktop/src/LyricsView';
import PlaybackProgress from '../../ui-desktop/src/PlaybackProgress';
import {usePlaybackFeed} from '../../ui-desktop/src/playbackFeed';
import {themePalette,defaultAppearance} from '../../ui-desktop/src/appearance';
import {native,request,artwork,emptyPlayback,errorText} from './bridge';
import type {Playback,Track as DesktopTrack,Palette} from '../../ui-desktop/src/types';
import './phone.css';
import PhoneSettings,{phoneDefaults,type PhonePreferences} from './PhoneSettings';

type Track=DesktopTrack&{album_id?:string};
function normalizePlayback(value:any,tracks:Track[]):Playback{return {...value,repeat:value.repeat??'none',queue:(value.queue??[]).map((entry:number|Track)=>typeof entry==='number'?tracks.find(t=>t.id===entry):entry).filter(Boolean)};}
type Collection={id:string;name:string;kind:string;track_ids:number[];complete?:boolean;revision?:string;artwork_path?:string;artwork_choice?:string};
type State={preferences?:PhonePreferences;music_haptics_active?:boolean;portraits?:Record<string,string>;tracks:Track[];collections:Collection[];nearby_macs?:{id:string;name:string}[];paired:boolean;pending_changes:number;conflicts:any[];sync:{state:string;message?:string;completed?:number;total?:number};storage_bytes:number;text_scale?:number;playback:Playback};
const blank:State={tracks:[],collections:[],paired:false,pending_changes:0,conflicts:[],sync:{state:'idle'},storage_bytes:0,playback:emptyPlayback as Playback};
function shuffleIds(ids:number[]){const a=[...ids];for(let i=a.length-1;i>0;i--){const j=Math.floor(Math.random()*(i+1));[a[i],a[j]]=[a[j],a[i]];}return a;}
function duration(s:number){return `${Math.floor(s/60)}:${Math.floor(s%60).toString().padStart(2,'0')}`;}
function Picture({track,className=''}:{track?:Track;className?:string}){return <span className={`phone-cover ${className}`}>{track?.cover_art_path?<img src={artwork(track.cover_art_path)} alt=""/>:<Icon name="Album"/>}</span>;}
function CollectionPicture({item,track}:{item:Collection;track?:Track}){const cover=item.artwork_path?artwork(item.artwork_path):collectionCover(item.artwork_choice);return cover?<span className="phone-cover"><img src={cover} alt=""/></span>:<Picture track={track}/>;}
function App(){
 const [state,setState]=useState(blank),[player,feed]=usePlaybackFeed(blank.playback),[destination,setDestination]=useState('home'),[category,setCategory]=useState('albums'),[query,setQuery]=useState(''),[detail,setDetail]=useState<{title:string;ids:number[];collection?:Collection}|null>(null),[view,setView]=useState<'normal'|'queue'|'lyrics'>('normal'),[devices,setDevices]=useState(false),[settings,setSettings]=useState(false),[error,setError]=useState(''),[palette,setPalette]=useState<Palette|null>(null),[lyricPreview,setLyricPreview]=useState<Track|undefined>(undefined),[songActions,setSongActions]=useState<Track|null>(null),[newPlaylist,setNewPlaylist]=useState(false),[name,setName]=useState(''),[selected,setSelected]=useState<number[]>([]),[editingPlaylist,setEditingPlaylist]=useState<string|undefined>(),[editingRevision,setEditingRevision]=useState<string|undefined>(),[playlistTarget,setPlaylistTarget]=useState<Track|null>(null),[queuePicker,setQueuePicker]=useState(false),[rowLimit,setRowLimit]=useState(60),[playlistQuery,setPlaylistQuery]=useState('');
 const preferences=state.preferences??phoneDefaults;
 const {expanded,closing,sheet,open:openSheet,close:closeSheet}=usePhonePlayerSheet(preferences.animations);
 const [navigationRevision,setNavigationRevision]=useState(0);
 const skipIntent=useRef<{direction:number;at:number}|undefined>(undefined);
 const previousView=useRef<{expanded:boolean;view:typeof view}|undefined>(undefined);
 const previousSong=useRef<{expanded:boolean;entry:number|undefined;index:number|null}|undefined>(undefined);
 const queueScroll=useRef(0);
 const pageBody=useRef<HTMLDivElement>(null),pageContent=useRef<HTMLDivElement>(null),pendingScroll=useRef<number|null>(null),direction=useRef(1);const catalogue=useRef<Track[]>([]);const content=useRef<HTMLElement>(null),scrolls=useRef<Record<string,number>>({}),destinationDetails=useRef<Record<string,typeof detail>>({}),browserScrolls=useRef<Record<string,number>>({}),playerRef=useRef(player);playerRef.current=player;
 useEffect(()=>{const haptic=(event:MouseEvent)=>{const button=(event.target as HTMLElement)?.closest('button');if(!button||button.disabled||button.closest('.heart,.reorder-handle,.remove-entry')||button.closest('.phone-settings'))return;feedback(button.closest('nav')?'selection':'light');};document.addEventListener('click',haptic);return()=>document.removeEventListener('click',haptic);},[]);
 async function refresh(){const value=await request({action:'snapshot'}) as State;catalogue.current=value.tracks;value.playback=normalizePlayback(value.playback,value.tracks);setState(value);feed(value.playback);}
 async function perform(action:Record<string,unknown>){if(action.action==='next'||action.action==='previous')skipIntent.current={direction:action.action==='next'?1:-1,at:performance.now()};setError('');try{const value=await request(action);if(value.tracks){catalogue.current=value.tracks;value.playback=normalizePlayback(value.playback,value.tracks);setState(value);feed(value.playback);}else if(value.queue){feed(normalizePlayback(value,catalogue.current));}return value;}catch(e){setError(errorText(e));throw e;}}
 useEffect(()=>{if(player.error)setError(errorText(player.error));},[player.error]);
 useEffect(()=>{void refresh().catch(e=>setError(errorText(e)));if(!native)return;let gone=false,off:(()=>void)|undefined;void addPluginListener<{json:string}>('companion','playback',value=>{if(!gone){const p=JSON.parse(value.json);feed(normalizePlayback(p,catalogue.current));if(p.text_scale)setState(s=>s.text_scale===p.text_scale?s:{...s,text_scale:p.text_scale});}}).then(listener=>{if(gone)void listener.unregister();else off=()=>void listener.unregister();}).catch(e=>setError(errorText(e)));return()=>{gone=true;off?.();};},[]);
 useEffect(()=>{if(!native)return;let gone=false,off:(()=>void)|undefined;void addPluginListener('companion','library',()=>{void refresh().catch(e=>setError(errorText(e)));}).then(l=>{if(gone)void l.unregister();else off=()=>void l.unregister();}).catch(()=>{});const visible=()=>{if(!document.hidden)void refresh().catch(()=>{});};document.addEventListener('visibilitychange',visible);return()=>{gone=true;off?.();document.removeEventListener('visibilitychange',visible);};},[]);

 useEffect(()=>{const cover=player.track?.cover_art_path;if(!cover)return;let gone=false;void coverPalette(cover).then(value=>{if(!gone&&value)setPalette(value);});return()=>{gone=true;};},[player.track?.cover_art_path]);
 const [dark,setDark]=useState(()=>matchMedia('(prefers-color-scheme: dark)').matches);useEffect(()=>{const media=matchMedia('(prefers-color-scheme: dark)'),update=()=>setDark(media.matches);media.addEventListener('change',update);return()=>media.removeEventListener('change',update);},[]);const effectiveDark=preferences.appearance==='system'?dark:preferences.appearance==='dark';const p=themePalette({...defaultAppearance,source:preferences.colors,mode:effectiveDark?'dark':'light'},palette,effectiveDark);
 const style={'--text-scale':String((state as any).text_scale??1),'--surface':p?.background??(effectiveDark?'#1b343e':'#c9e6e7'),'--background':p?.background??(effectiveDark?'#10262f':'#a5d1d8'),'--ink':p?.text??(effectiveDark?'#e3f2f2':'#142630'),'--accent':p?.accent??(effectiveDark?'#87ccd5':'#2b6d7e'),'--blob-color':p?.blob??'#91c1c9','--surface-color':p?.background??(effectiveDark?'#1b343e':'#c9e6e7'),'--accent-ink':p?.text??'#142630','--control-border':`color-mix(in srgb,${p?.text??'#142630'} 15%,transparent)`,'--control-surface':`color-mix(in srgb,${p?.blob??'#91c1c9'} 22%,${p?.background??'#bedde0'})`} as CSSProperties;
 useEffect(()=>{document.documentElement.style.fontSize=`${16*(state.text_scale??1)}px`;},[state.text_scale]);
 const entry=player.queue_index===null?player.track?.id:player.queue_entry_ids?.[player.queue_index]??player.track?.id;
 useLayoutEffect(()=>{
  const before=previousView.current;
  previousView.current={expanded,view};
  const node=pageBody.current;
  const changedView=before?.view!==view;
  if(expanded&&view==='queue'&&(!before?.expanded||changedView)){
   const queue=node?.querySelector<HTMLElement>('.phone-queue');
   if(queue)queue.scrollTop=queueScroll.current;
  }
  if(expanded&&before?.expanded&&changedView&&!closing&&node&&allowsPhoneMotion(preferences.animations)){
   return animateOwned(node,'phone-player-view',[{opacity:.88,transform:'translateY(5px)'},{opacity:1,transform:'none'}],180);
  }
 },[expanded,closing,view,preferences.animations]);
 useLayoutEffect(()=>{
  const before=previousSong.current;
  previousSong.current={expanded,entry,index:player.queue_index};
  const node=pageBody.current;
  if(!before||closing||!allowsPhoneMotion(preferences.animations)||node?.getAnimations().some(animation=>animation.id==='phone-player-view'))return;
  if(entry===undefined||before.entry===undefined||entry===before.entry||before.expanded!==expanded)return;
  const intent=skipIntent.current;
  skipIntent.current=undefined;
  const sign=intent&&performance.now()-intent.at<1500?intent.direction:
   player.queue_index!==null&&before.index!==null&&player.queue_index<before.index&&player.queue_index!==0?-1:1;
  const identities=expanded&&view==='normal'
   ?[...node?.querySelectorAll<HTMLElement>('.phone-playing-art,.phone-playing-identity')??[]]
   :expanded?[]:[document.querySelector<HTMLElement>('.mini-identify')];
  const cleanup=identities.flatMap(identity=>identity?[animateOwned(identity,'phone-song-handoff',[
   {opacity:.86,transform:`translateX(${sign*7}px)`},{opacity:1,transform:'none'},
  ],180)]:[]);
  return ()=>cleanup.forEach(cancel=>cancel());
 },[expanded,closing,view,entry,preferences.animations]);
 const tracks=useMemo(()=>[...state.tracks].sort(songOrder),[state.tracks]),byId=new Map(tracks.map(t=>[t.id,t])),albums=useMemo(()=>orderedAlbums(tracks),[tracks]);
 useLayoutEffect(()=>{if(pendingScroll.current!==null&&content.current){content.current.scrollTop=pendingScroll.current;pendingScroll.current=null;}const node=pageContent.current;if(node&&navigationRevision>0&&preferences.animations)return animateOwned(node,'phone-destination',[{opacity:.91,transform:`translateX(${direction.current*7}px)`},{opacity:1,transform:'none'}],180);},[navigationRevision,preferences.animations]);
 useScrollEdges(content,!expanded&&!settings&&!devices,`${destination}:${detail?.title}:${query}:${state.tracks.length}`);
 function navigate(next:string){if(destination===next)return;direction.current=['home','search','library'].indexOf(next)>['home','search','library'].indexOf(destination)?1:-1;if(content.current)scrolls.current[destination]=content.current.scrollTop;destinationDetails.current[destination]=detail;pendingScroll.current=scrolls.current[next]??0;setDestination(next);setDetail(destinationDetails.current[next]??null);setNavigationRevision(n=>n+1);}
 async function play(ids:number[],index=0){await perform({action:'play',ids,index});}
 function openDetail(value:NonNullable<typeof detail>){direction.current=1;browserScrolls.current[destination]=content.current?.scrollTop??0;pendingScroll.current=0;setDetail(value);setNavigationRevision(n=>n+1);}
 function closeDetail(){direction.current=-1;pendingScroll.current=browserScrolls.current[destination]??0;setDetail(null);setNavigationRevision(n=>n+1);}
 useEffect(()=>setRowLimit(60),[destination,category,query,detail?.title]);
 useEffect(()=>{if(newPlaylist)return;setDetail(old=>{if(!old?.collection)return old;const fresh=state.collections.find(c=>c.id===old.collection!.id);if(!fresh||fresh.name===old.title&&fresh.revision===old.collection.revision&&JSON.stringify(fresh.track_ids)===JSON.stringify(old.ids))return old;return {...old,title:fresh.name,ids:fresh.track_ids,collection:fresh};});},[state.collections,newPlaylist]);
 function rows(list:Track[]){return <div className="phone-songs">{list.slice(0,rowLimit).map((t,i)=><SwipeSong key={`${t.id}:${i}`} onNext={()=>void perform({action:'queue_add',ids:[t.id],next:true})}><div className="phone-song" onContextMenu={e=>{e.preventDefault();setSongActions(t);}}><button className="song-open" onClick={()=>void play(list.map(t=>t.id),i)}><Picture track={t}/><span><strong>{t.title}</strong><small>{t.artist}</small></span></button><button className="icon-button" aria-label={`Actions for ${t.title}`} onClick={()=>setSongActions(t)}><Icon name="3dots_(nocircle)"/></button><button className="icon-button heart" aria-label={`${t.favorited?'Unfavorite':'Favorite'} ${t.title}`} onClick={()=>void perform({action:'favorite',id:t.id,value:!t.favorited})}><Icon name={t.favorited?'hearted':'noheart'}/></button></div></SwipeSong>)}{list.length>rowLimit&&<button className="filled load-more" onClick={()=>setRowLimit(n=>n+60)}>Show more songs</button>}</div>;}
 function albumGrid(groups:Track[][]){return <div className="phone-grid">{groups.map(group=><button className="album-tile" key={String(group[0].album_id??group[0].id)} onClick={()=>openDetail({title:group[0].album,ids:group.map(t=>t.id)})}><Picture track={group[0]}/><strong>{group[0].album}</strong><small>{group[0].artist}</small></button>)}</div>;}
 const shown=query.trim()?tracks.filter(t=>`${t.title} ${t.artist} ${t.album}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())):tracks;
 const detailTracks=detail?.ids.flatMap(id=>{const track=byId.get(id);return track?[track]:[];})??[];
 const current=player.track?{...player.track,favorited:byId.get(player.track.id)?.favorited??player.track.favorited}:undefined;
 function openPlayer(next:typeof view,preview?:Track,opener?:HTMLElement){
  setView(next);
  setLyricPreview(preview);
  openSheet(opener);
 }
 const cycle=()=>setView(v=>v==='normal'?'queue':v==='queue'?'lyrics':'normal');
 const backSwipe=useSwipe(d=>{if(d==='right'&&detail)closeDetail();},'horizontal',detail?'edge':'off');
 const mainRef=useCallback((node:HTMLElement|null)=>{content.current=node;backSwipe.ref(node);},[backSwipe.ref]);
 const miniSwipe=useSwipe(d=>{if(d==='up'){openPlayer('normal');}else if(d==='left')void perform({action:'next'});else if(d==='right')void perform({action:'previous'});},'both');
 const dismissSwipe=useSwipe((d,offset)=>{if(d==='down')closeSheet(offset);},'vertical','all',true);
 const playerSwipe=useSwipe((d,offset)=>{
  if(d==='down')closeSheet(offset);
  else if(d==='left')void perform({action:'next'});
  else if(d==='right')void perform({action:'previous'});
 },'both',view==='normal'?'all':'off',true);
 const playerBodyRef=useCallback((node:HTMLDivElement|null)=>{pageBody.current=node;playerSwipe.ref(node);},[playerSwipe.ref]);
 const transport=<div className="phone-transport"><button className="icon-button heart" aria-label={current?.favorited?'Unfavorite song':'Favorite song'} disabled={!current} onClick={()=>current&&void perform({action:'favorite',id:current.id,value:!current.favorited})}><Icon motion={preferences.animations} name={current?.favorited?'hearted':'noheart'}/></button><button className="icon-button" aria-label="Previous song" onClick={()=>void perform({action:'previous'})}><Icon name="prev"/></button><button className="icon-button play" aria-label={player.is_playing?'Pause':'Play'} onClick={()=>void perform({action:'toggle'})}><Icon motion={preferences.animations} name={(player as any).preparing?'sync':player.is_playing?'pause':'play'}/></button><button className="icon-button" aria-label="Next song" onClick={()=>void perform({action:'next'})}><Icon name="next"/></button><button className="icon-button" aria-label={`Show ${view==='normal'?'Up next':view==='queue'?'Lyrics':'Now playing'}`} onClick={cycle}><Icon motion={preferences.animations} name={view==='normal'?'playerQueue':view==='queue'?'playerLyrics':'playerPlaying'}/></button></div>;
 return <div className="phone-app" style={style} data-reduced-motion={!preferences.animations||undefined}>
 <header inert={expanded||devices||settings||newPlaylist||songActions||playlistTarget?true:undefined}><span className="brand"><Icon name="Album"/>musicOS</span><button className="icon-button" aria-label="Settings" onClick={()=>{void refresh();setSettings(true);}}><Icon name="Settings"/></button></header>
 <main className="phone-scroll-edges" {...backSwipe} ref={mainRef} inert={expanded||devices||settings||newPlaylist||songActions||playlistTarget?true:undefined}><div ref={pageContent} className="phone-page">
 {detail?<><button className="filled compact" onClick={closeDetail}><Icon name="Back"/>Back</button><div className="phone-detail"><Picture track={detailTracks[0]}/><h1>{detail.title}</h1><p>{detailTracks.length===detail.ids.length?`${detailTracks.length} ${detailTracks.length===1?'song':'songs'}`:`${detailTracks.length} of ${detail.ids.length} songs offline`}</p><div className="actions"><button className="filled" disabled={!detailTracks.length} onClick={()=>void play(detailTracks.map(track=>track.id))}><Icon name="play"/>Play</button><button className="filled" disabled={!detailTracks.length} onClick={()=>void play(shuffleIds(detailTracks.map(track=>track.id)))}><Icon name="ShuffleON"/>Shuffle</button>{detail.collection?.kind==='manual'&&detail.collection.complete!==false&&<button className="filled compact" onClick={()=>{setEditingPlaylist(detail.collection?.id);setEditingRevision(detail.collection?.revision??'');setName(detail.title);setSelected(detail.ids);setNewPlaylist(true);}}>Edit</button>}</div></div>{rows(detailTracks)}</>:<>
 <h1>{destination==='home'?'Home':destination==='search'?'Search':'Library'}</h1>
 {destination==='home'?<>
 {current&&<section><h2>Continue listening</h2><div className="continue-card"><Picture track={current}/><button onClick={event=>openPlayer('normal',undefined,event.currentTarget)}><strong>{current.title}</strong><small>{current.artist} · {duration(player.position)}</small></button><button className="icon-button" aria-label={player.is_playing?'Pause':'Resume'} onClick={()=>void perform({action:'toggle'})}><Icon motion={preferences.animations} name={(player as any).preparing?'sync':player.is_playing?'pause':'play'}/></button></div></section>}
 <section><div className="section-title"><h2>Collections</h2><button className="filled compact" onClick={()=>{setEditingPlaylist(undefined);setName('');setSelected([]);setNewPlaylist(true);}}><Icon name="Plus"/>New</button></div><div className="collection-list"><button onClick={()=>openDetail({title:'Favorites',ids:tracks.filter(t=>t.favorited).map(t=>t.id)})}><span className="collection-symbol"><Icon name="hearted"/></span><span><strong>Favorites</strong><small>{tracks.filter(t=>t.favorited).length} songs</small></span><Icon name="Back" className="chevron-right"/></button>{state.collections.map(c=><button key={c.id} onClick={()=>openDetail({title:c.name,ids:c.track_ids,collection:c})}><CollectionPicture item={c} track={byId.get(c.track_ids[0])}/><span><strong>{c.name}</strong><small>{c.track_ids.length} songs{c.complete===false?' · Partly downloaded':''}</small></span><Icon name="Back" className="chevron-right"/></button>)}</div></section>
 <section><h2>Albums</h2>{albumGrid(albums.slice(0,8))}</section>
 {!tracks.length&&<div className="phone-empty"><Icon name="Library"/><h2>Your music, offline.</h2><button className="filled" onClick={()=>setDevices(true)}>Pair your Mac</button></div>}
 </>:destination==='search'?<><input type="search" aria-label="Search downloaded music" placeholder="Songs, artists, albums" value={query} onChange={e=>setQuery(e.target.value)}/>{rows(shown)}</>:<><div className="phone-categories">{['albums','artists','songs','favorites','collections'].map(c=><button key={c} aria-pressed={category===c} onClick={()=>setCategory(c)}>{c[0].toUpperCase()+c.slice(1)}</button>)}</div>{category==='albums'?albumGrid(albums):category==='artists'?<div className="collection-list">{[...new Set(tracks.map(t=>t.artist))].sort(artistOrder).map(artist=><button key={artist} onClick={()=>openDetail({title:artist,ids:tracks.filter(t=>t.artist===artist).sort(artistTrackOrder).map(t=>t.id)})}><span className="phone-cover artist-portrait">{state.portraits?.[artist]?<img src={artwork(state.portraits[artist])} alt=""/>:<Icon name="Artist"/>}</span><strong>{artist}</strong></button>)}</div>:category==='collections'?<div className="collection-list">{state.collections.map(c=><button key={c.id} onClick={()=>openDetail({title:c.name,ids:c.track_ids,collection:c})}><CollectionPicture item={c} track={byId.get(c.track_ids[0])}/><strong>{c.name}</strong></button>)}</div>:rows(category==='favorites'?tracks.filter(t=>t.favorited):tracks)}</>}
 </>}
 </div></main>
 <footer inert={expanded||devices||settings||newPlaylist||songActions||playlistTarget?true:undefined}>{current&&<div className="phone-mini" {...miniSwipe}><button className="mini-identify" onClick={event=>openPlayer('normal',undefined,event.currentTarget)}><Picture track={current}/><span><strong>{current.title}</strong><small>{current.artist}</small></span></button><button className="icon-button" aria-label={player.is_playing?'Pause':'Resume'} onClick={()=>void perform({action:'toggle'})}><Icon motion={preferences.animations} name={(player as any).preparing?'sync':player.is_playing?'pause':'play'}/></button><button className="icon-button" aria-label="Up next" onClick={event=>openPlayer('queue',undefined,event.currentTarget)}><Icon name="playerQueue"/></button><div className="mini-progress" aria-hidden="true" style={{'--mini-progress':`${Math.min(100,player.position/(current.duration||1)*100)}%`} as CSSProperties}/></div>}<nav>{[['home','Home'],['search','Search'],['library','Library']].map(([id,icon])=><button aria-current={destination===id?'page':undefined} key={id} onClick={()=>navigate(id)}><Icon name={id==='search'?'search':icon}/><span>{icon}</span></button>)}</nav></footer>
 {expanded&&<section ref={sheet} className="phone-player" data-swipe-closing={closing||undefined} role="dialog" aria-modal="true" aria-label="Player" inert={closing||queuePicker||songActions||newPlaylist||playlistTarget?true:undefined}><div className="player-top" {...dismissSwipe}><button className="icon-button" aria-label="Close player" onClick={()=>closeSheet()}><Icon name="Back" className="chevron-down"/></button><span>{view==='normal'?'Now playing':view==='queue'?'Up next':'Lyrics'}</span><button className="icon-button" aria-label="Audio output" onClick={()=>void perform({action:'route'})}><Icon name="device"/></button></div><div {...playerSwipe} ref={playerBodyRef} className={`phone-player-content ${view}`} onScrollCapture={event=>{const target=event.target;if(target instanceof HTMLElement&&target.classList.contains('phone-queue'))queueScroll.current=target.scrollTop;}}>
 {view==='normal'?<div className="phone-playing"><div className="phone-playing-art"><Picture track={current}/></div><div className="phone-playing-identity"><h1>{current?.title??'Nothing playing'}</h1>{current&&<p>{current.artist} · {current.album}</p>}</div></div>:view==='lyrics'?<LyricsView renderMenu={()=>null} track={lyricPreview??current} player={player} active={!closing} onSeek={s=>perform({action:'seek',position:s})} onPlay={()=>play((lyricPreview??current)?[(lyricPreview??current)!.id]:[])} variant="panel"/>:<QueueEditor player={player} onAction={perform} onAdd={()=>setQueuePicker(true)}/>}
 </div><div className="player-bottom"><div className="phone-playback-options"><button className="icon-button" aria-label="Shuffle queue" aria-pressed={player.shuffle} onClick={()=>void perform({action:'shuffle'})}><Icon motion={preferences.animations} name={player.shuffle?'ShuffleON':'ShuffleOFF'}/></button><button className="icon-button" aria-label="Repeat" aria-pressed={player.repeat!=='none'} onClick={()=>void perform({action:'repeat'})}><Icon motion={preferences.animations} name={player.repeat==='one'?'RepeatONE':player.repeat==='all'?'RepeatON':'RepeatOFF'}/></button></div><PlaybackProgress trackId={current?.id} position={player.position} duration={current?.duration??0} seek={async s=>{await perform({action:'seek',position:s});}}/>{transport}</div></section>}
 {settings&&<PhoneSettings preferences={preferences} musicHapticsActive={state.music_haptics_active??false} onChange={(key,value)=>void perform({action:'settings',key,value})} onComputer={()=>{setSettings(false);setDevices(true);}} paired={state.paired} onClose={()=>setSettings(false)}/>}
 {devices&&<ComputerSheet paired={state.paired} onClose={()=>setDevices(false)} onLibraryChange={refresh}/>}
 {newPlaylist&&<PlaylistEditor tracks={tracks} initialName={name} initialIds={selected} editing={!!editingPlaylist} onClose={()=>setNewPlaylist(false)} onSave={async(name,ids)=>{await perform({action:'playlist',id:editingPlaylist,base_revision:editingPlaylist?editingRevision:undefined,name,track_ids:ids});setNewPlaylist(false);if(editingPlaylist&&detail?.collection?.id===editingPlaylist)setDetail({...detail,title:name,ids});}}/>}
 {queuePicker&&<AddQueueSongs tracks={tracks} onClose={()=>setQueuePicker(false)} onAdd={id=>perform({action:'queue_add',ids:[id]}).then(()=>feedback('drop'))}/>}

 {playlistTarget&&<section className="song-actions-sheet" role="dialog" aria-modal="true" aria-label="Choose playlist"><h2>Add to playlist</h2><input type="search" aria-label="Find playlist" placeholder="Find a playlist" value={playlistQuery} onChange={e=>setPlaylistQuery(e.target.value)}/>{state.collections.filter(c=>c.kind==='manual'&&c.complete!==false&&c.name.toLocaleLowerCase().includes(playlistQuery.toLocaleLowerCase())).map(c=><button key={c.id} className="filled" onClick={()=>void perform({action:'playlist',id:c.id,name:c.name,track_ids:[...c.track_ids,playlistTarget.id]}).then(()=>setPlaylistTarget(null))}><Icon name="Playlist"/>{c.name}</button>)}{!state.collections.some(c=>c.kind==='manual'&&c.complete!==false)&&<p>Create a playlist first, or finish downloading an existing one.</p>}<button className="filled" onClick={()=>setPlaylistTarget(null)}>Cancel</button></section>}
 {songActions&&<section className="song-actions-sheet" role="dialog" aria-modal="true" aria-label={`Actions for ${songActions.title}`}><h2>{songActions.title}</h2><p>{songActions.artist}</p><button className="filled" onClick={()=>{void play([songActions.id]);setSongActions(null);}}><Icon name="play"/>Play</button><button className="filled" onClick={()=>{void perform({action:'queue_add',ids:[songActions.id],next:true});setSongActions(null);}}><Icon name="Play Next"/>Play next</button><button className="filled" onClick={()=>{void perform({action:'queue_add',ids:[songActions.id]});setSongActions(null);}}><Icon name="Add to queue"/>Add to queue</button><button className="filled" onClick={()=>{openPlayer('lyrics',songActions);setSongActions(null);}}><Icon name="Lyrics"/>Lyrics</button><button className="filled" onClick={()=>{setEditingPlaylist(undefined);setName('');setSelected([songActions.id]);setNewPlaylist(true);setSongActions(null);}}><Icon name="Playlist"/>New playlist with song</button><button className="filled" onClick={()=>{setPlaylistTarget(songActions);setSongActions(null);}}><Icon name="Plus"/>Add to playlist</button><button className="filled" onClick={()=>setSongActions(null)}>Close</button></section>}
 {error&&<div className="phone-error" role="alert"><span>{error}</span><button aria-label="Dismiss error" onClick={()=>setError('')}>×</button></div>}
 </div>;
}
createRoot(document.getElementById('root')!).render(<App/>);
