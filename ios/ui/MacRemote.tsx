import {useEffect,useRef,useState} from 'react';
import {AssetIcon as Icon} from '../../ui-desktop/src/AssetIcon';
import {errorText,request} from './bridge';

type Song={id:number;title:string;artist:string;album:string;duration:number;available:boolean;has_cover:boolean};
type Release={id:string;title:string;artist:string;count:number};
type QueueItem={entry:number;track:Song};
type RemoteState={version:number;server_id:string;name:string;track?:Song;entry?:number;queue:QueueItem[];queue_revision:number;is_playing:boolean;position:number};
type Page={tracks:Song[];albums:Release[];selected?:{track_ids:number[]};more:boolean;offset:number;version:number};
type Command={kind:'toggle'|'next'|'previous'}|{kind:'play';ids:number[];start:number}|{kind:'seek';position:number;entry:number}
 |{kind:'add';id:number;next:boolean;revision:number}|{kind:'remove';entry:number;revision:number}|{kind:'move';entry:number;target:number;revision:number};

export default function MacRemote({onClose}:{onClose:()=>void}){
 const [state,setState]=useState<RemoteState>(),[stale,setStale]=useState(true),[busy,setBusy]=useState(false);
 const [statusError,setStatusError]=useState(''),[actionError,setActionError]=useState('');
 const [tab,setTab]=useState<'library'|'queue'>('library'),[query,setQuery]=useState(''),[album,setAlbum]=useState<Release>();
 const [page,setPage]=useState<Page>(),[searching,setSearching]=useState(false),[image,setImage]=useState('');
 const [seek,setSeek]=useState<number>();
 const generation=useRef(0),working=useRef(false),server=useRef<string>();

 function receive(next:RemoteState){
  if(next.version!==1||!next.server_id)throw new Error('Mac Remote needs an updated copy of musicOS on your Mac.');
  if(server.current&&server.current!==next.server_id)throw new Error('The paired Mac changed. Reopen Mac Remote.');
  server.current=next.server_id;setState(next);setStale(false);setStatusError('');
 }
 useEffect(()=>{
  let live=true,timer=0;
  async function poll(){
   const token=generation.current;
   if(!document.hidden&&!working.current){
    try{
     const next=await request({action:'mac_remote_state'});
     if(live&&token===generation.current)receive(next);
    }catch(error){
     if(live&&token===generation.current){setStale(true);setStatusError(errorText(error));}
    }
   }
   if(live)timer=window.setTimeout(poll,850);
  }
  function visibility(){if(document.hidden){generation.current++;setStale(true);}}
  document.addEventListener('visibilitychange',visibility);
  void poll();
  return()=>{live=false;clearTimeout(timer);document.removeEventListener('visibilitychange',visibility);};
 },[]);
 useEffect(()=>{
  setPage(undefined);
  if(stale||tab!=='library')return;
  let live=true;
  const timer=window.setTimeout(()=>{
   setSearching(true);
   void request({action:'mac_remote_library',query,album:album?.id,offset:0}).then((next:Page)=>{
    if(live&&next.version===1)setPage(next);
   }).catch(error=>{if(live)setActionError(errorText(error));}).finally(()=>{if(live)setSearching(false);});
  },160);
  return()=>{live=false;clearTimeout(timer);setSearching(false);};
 },[stale,tab,query,album?.id]);
 useEffect(()=>{
  setImage('');
  if(stale||!state?.track?.has_cover)return;
  let live=true;
  void request({action:'mac_remote_cover',id:state.track.id}).then(result=>{if(live)setImage(result.image??'');}).catch(()=>{});
  return()=>{live=false;};
 },[stale,state?.track?.id,state?.track?.has_cover]);

 async function send(command:Command){
  if(stale||working.current)return;
  working.current=true;++generation.current;setBusy(true);setActionError('');
  try{receive(await request({action:'mac_remote_command',command}));}
  catch(error){setStale(true);setActionError(errorText(error));}
  finally{working.current=false;setBusy(false);}
 }
 async function more(){
  if(!page||stale||searching)return;
  setSearching(true);
  try{
   const next:Page=await request({action:'mac_remote_library',query,album:album?.id,offset:page.offset+60});
   setPage(current=>current===page?{...next,tracks:[...page.tracks,...next.tracks],albums:[...page.albums,...next.albums]}:current);
  }catch(error){setActionError(errorText(error));}
  finally{setSearching(false);}
 }

 const disabled=stale||busy,revision=state?.queue_revision??0;
 return <section className="phone-sheet phone-remote" role="dialog" aria-modal="true" aria-label="Mac Remote">
  <header><div><h1>Playing on Mac</h1><small>{state?.name??'Mac Remote'}</small></div>
   <button className="filled" aria-label="Return to iPhone playback" onClick={onClose}><Icon name="controlClose"/></button></header>
  {stale&&<p role="status" className="remote-message">{statusError||'Connecting to your Mac…'}{state&&' Showing the last known state.'}</p>}
  {actionError&&<p role="alert" className="remote-message">{actionError}</p>}
  <div className="remote-now">
   {image&&<img src={image} alt=""/>}
   <div><strong>{state?.track?.title??'Nothing playing'}</strong><small>{state?.track?.artist}</small></div>
   <div className="remote-actions">
    <button className="filled" disabled={disabled} aria-label="Previous song on Mac" onClick={()=>void send({kind:'previous'})}><Icon name="prev"/></button>
    <button className="filled" disabled={disabled||!state?.track} aria-label={state?.is_playing?'Pause Mac':'Resume Mac'} onClick={()=>void send({kind:'toggle'})}><Icon name={state?.is_playing?'pause':'play'}/></button>
    <button className="filled" disabled={disabled} aria-label="Next song on Mac" onClick={()=>void send({kind:'next'})}><Icon name="next"/></button>
   </div>
   {state?.track&&state.entry!==undefined&&state.track.duration>0&&<label className="remote-seek">
    <span>Playback position</span><input aria-label="Playback position on Mac" type="range" min={0} max={state.track.duration} step=".25"
     value={seek??state.position} disabled={disabled} onChange={event=>setSeek(Number(event.currentTarget.value))}
     onPointerUp={event=>{const entry=state.entry;if(entry!==undefined)void send({kind:'seek',position:Number(event.currentTarget.value),entry});setSeek(undefined);}}
     onKeyUp={event=>{const entry=state.entry;if(entry!==undefined&&event.key.match(/Arrow|Home|End/)){void send({kind:'seek',position:Number(event.currentTarget.value),entry});setSeek(undefined);}}}
     onPointerCancel={()=>setSeek(undefined)}/>
   </label>}
  </div>
  <div className="remote-actions" role="group" aria-label="Mac pages">
   <button className="filled" aria-pressed={tab==='library'} onClick={()=>setTab('library')}>Library</button>
   <button className="filled" aria-pressed={tab==='queue'} onClick={()=>setTab('queue')}>Up next</button>
  </div>
  {tab==='library'?<div className="remote-list">
   {album?<div className="remote-actions">
    <button className="filled" onClick={()=>setAlbum(undefined)}><Icon name="Back"/>Albums</button>
    <button className="filled" disabled={disabled||!page?.selected?.track_ids.length} onClick={()=>{if(page?.selected)void send({kind:'play',ids:page.selected.track_ids,start:0});}}>Play album</button>
   </div>:<input aria-label="Search Mac library" placeholder="Search Mac library" type="search" value={query}
    onChange={event=>{setActionError('');setQuery(event.currentTarget.value);}}/>}
   {!album&&!!page?.albums.length&&<><h2>Albums</h2>{page.albums.map(release=><button className="remote-release filled" key={release.id} onClick={()=>setAlbum(release)}>
    <span><strong>{release.title}</strong><small>{release.artist} · {release.count} songs</small></span><Icon name="Album"/>
   </button>)}</>}
   {!!page?.tracks.length&&<><h2>{album?.title??'Songs'}</h2>{page.tracks.map(track=><div className="remote-row" key={track.id}>
    <span><strong>{track.title}</strong><small>{track.artist}</small></span>
    <button className="filled" disabled={disabled||!track.available} aria-label={`Play ${track.title} on Mac`}
     onClick={()=>void send({kind:'play',ids:page.selected?.track_ids.includes(track.id)?page.selected.track_ids:[track.id],start:Math.max(0,page.selected?.track_ids.indexOf(track.id)??0)})}><Icon name="play"/></button>
    <button className="filled" disabled={disabled||!track.available} aria-label={`Add ${track.title} to Mac queue`}
     onClick={()=>void send({kind:'add',id:track.id,next:false,revision})}><Icon name="Plus"/></button>
   </div>)}</>}
   {searching&&<p role="status">Loading library…</p>}
   {!searching&&page&&!page.tracks.length&&!page.albums.length&&<p>No matching music.</p>}
   {page?.more&&<button className="filled" disabled={disabled||searching} onClick={()=>void more()}>Show more</button>}
  </div>:<div className="remote-list">
   {!state?.queue.length&&<p>No songs up next.</p>}
   {state?.queue.map(({entry,track},index)=><div className="remote-row" key={entry}>
    <span><strong>{track.title}</strong><small>{track.artist}</small></span>
    <button className="filled" disabled={disabled||index===0} aria-label={`Move ${track.title} earlier`} onClick={()=>void send({kind:'move',entry,target:state.queue[index-1].entry,revision})}>↑</button>
    <button className="filled" disabled={disabled||index===state.queue.length-1} aria-label={`Move ${track.title} later`} onClick={()=>void send({kind:'move',entry,target:state.queue[index+1].entry,revision})}>↓</button>
    <button className="filled" disabled={disabled} aria-label={`Remove ${track.title} from Mac queue`} onClick={()=>void send({kind:'remove',entry,revision})}><Icon name="controlClose"/></button>
   </div>)}
  </div>}
 </section>;
}
