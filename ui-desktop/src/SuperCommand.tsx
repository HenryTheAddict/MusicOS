import {useEffect,useMemo,useRef,useState} from 'react';
import {Cover,Icon} from './components';
import {call,errorText} from './bridge';
import {trackCredits} from './credits';
import {searchScore,groupMusicResults} from './commandSearch';
import type {Album,Artist,Library,SavedCollection} from './types';
interface Result {group:string;score:number;id:string;title:string;detail:string;icon?:string;cover?:string|null;run:()=>void}
export default function SuperCommand({library,collections,isPlaying,hasTrack,onClose,onPlay,onAlbum,onArtist,onCollection,onNavigate,onPlayback,onQueue,onCreate}:{library:Library;collections:SavedCollection[];isPlaying:boolean;hasTrack:boolean;onClose:()=>void;onPlay:(id:number)=>void;onAlbum:(album:Album)=>void;onArtist:(artist:Artist)=>void;onCollection:(item:SavedCollection)=>void;onNavigate:(destination:string)=>void;onPlayback:(action:'toggle'|'next'|'previous')=>void;onQueue:()=>void;onCreate:(items:SavedCollection[])=>void}){
 const dialog=useRef<HTMLDialogElement>(null),input=useRef<HTMLInputElement>(null);
 const handlers=useRef({onPlay,onAlbum,onArtist,onCollection,onNavigate,onPlayback,onQueue});handlers.current={onPlay,onAlbum,onArtist,onCollection,onNavigate,onPlayback,onQueue};
 const [query,setQuery]=useState(''),[active,setActive]=useState(0),[creating,setCreating]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{const opener=document.activeElement as HTMLElement|null;const modal=dialog.current;modal?.showModal();input.current?.focus();return()=>{modal?.close();if(opener?.isConnected)opener.focus();};},[]);
 const results=useMemo(()=>{
   const needle=query.trim().toLocaleLowerCase();const matches=(text:string)=>needle.split(/\s+/).every(word=>text.toLocaleLowerCase().includes(word));
   const actions:Omit<Result,'group'|'score'>[]=[...(hasTrack?[{id:'toggle',title:isPlaying?'Pause music':'Play music',detail:'Playback',icon:isPlaying?'pause':'play',run:()=>handlers.current.onPlayback('toggle')},{id:'next',title:'Next song',detail:'Playback',icon:'next',run:()=>handlers.current.onPlayback('next')},{id:'previous',title:'Previous song',detail:'Playback',icon:'prev',run:()=>handlers.current.onPlayback('previous')}]:[]),{id:'new-playlist',title:'Create playlist',detail:'Your collection, your order',icon:'Playlist',run:()=>{setCreating(true);setQuery('');setError('');}},{id:'queue',title:'Open Up next',detail:'Listening queue and DJmode',icon:'queue',run:()=>handlers.current.onQueue()},{id:'collections',title:'Manage collections',detail:'Playlists, Sparklists, and Smart playlists',icon:'Playlist',run:()=>handlers.current.onNavigate('Playlists')},...['Albums','Artists','Genres','Songs','Favorites','Settings','Devices'].map(title=>({id:`navigate-${title}`,title:`Go to ${title}`,detail:'Navigation',icon:title==='Artists'?'Artist':'Album',run:()=>handlers.current.onNavigate(title)}))];
   const commands=actions.filter(r=>!needle||matches(`${r.title} ${r.detail}`)).map(r=>({...r,group:'Commands',score:0}));
   const found:Result[]=[];
   if(needle){
     found.push(...library.tracks.filter(t=>t.available&&matches(`${t.title} ${t.artist} ${t.album}`)).map(t=>({group:'Songs',score:searchScore(needle,t.title),id:`song-${t.id}`,title:t.title,detail:trackCredits(t).join(' + '),cover:t.cover_art_path,icon:'Song',run:()=>handlers.current.onPlay(t.id)})));
     found.push(...library.albums.filter(a=>matches(`${a.title} ${a.artist}`)).map(a=>({group:'Albums',score:searchScore(needle,a.title),id:a.id,title:a.title,detail:a.artist.replaceAll(';',' +'),cover:a.cover,icon:'Album',run:()=>handlers.current.onAlbum(a)})));
     found.push(...library.artists.filter(a=>matches(a.name)).map(a=>({group:'Artists',score:searchScore(needle,a.name),id:a.id,title:a.name,detail:'Artist',cover:a.portrait,icon:'Artist',run:()=>handlers.current.onArtist(a)})));
   }
   found.push(...collections.filter(c=>matches(c.name)).map(c=>({group:'Collections',score:searchScore(needle,c.name),id:c.id,title:c.name,detail:`${c.kind==='audio'?'Sparklist':c.spark?'Smart playlist':'Playlist'} · ${c.track_ids.length} songs`,icon:c.kind==='audio'?'Sparklist':'Playlist',run:()=>handlers.current.onCollection(c)})));
   return [...groupMusicResults(found),...commands];
 },[query,library,collections,isPlaying,hasTrack]);
 useEffect(()=>setActive(0),[query]);
 useEffect(()=>setActive(index=>Math.max(0,Math.min(index,results.length-1))),[results.length]);
 useEffect(()=>{dialog.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({block:'nearest'});},[active,results.length]);
 function select(result:Result){result.run();if(result.id!=='new-playlist')onClose();}
 async function create(){if(busy)return;setBusy(true);setError('');try{onCreate(await call('collections',{request:{action:'create',name:query}}));onClose();}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <dialog ref={dialog} className="supercommand" aria-label="SuperCommand" onCancel={e=>{e.preventDefault();onClose();}} onClick={e=>{if(e.target===e.currentTarget)onClose();}} onKeyDown={e=>{
  if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();setActive(index=>Math.max(0,Math.min(results.length-1,index+(e.key==='ArrowDown'?1:-1))));}
  if(e.key==='Enter'&&!e.nativeEvent.isComposing){e.preventDefault();if(creating)void create();else if(results[active])select(results[active]);}
 }}><header><Icon name="search"/><input ref={input} role="combobox" aria-label={creating?'Playlist name':'SuperCommand search'} aria-expanded={!creating} aria-controls="supercommand-results" aria-activedescendant={!creating&&results[active]?`command-${active}`:undefined} placeholder={creating?'Name your playlist…':'Search music or type a command…'} value={query} onChange={e=>setQuery(e.target.value)}/><button aria-label="Close SuperCommand" onClick={onClose}>Esc</button></header>
 {creating?<section className="command-create"><h2>Create playlist</h2><p>It will appear in your stuff.</p><button className="primary" disabled={busy||!query.trim()} onClick={()=>void create()}>{busy?'Creating…':'Create playlist'}</button>{error&&<p role="alert">{error}</p>}<button onClick={()=>{setCreating(false);setQuery('');}}>Back to commands</button></section>:<div id="supercommand-results" className="command-results" role="listbox" aria-label="Commands and music">{[...new Set(results.map(result=>result.group))].map(group=><section className="command-group" key={group} role="group" aria-label={group}><h3 aria-hidden="true">{group}</h3>{results.map((result,index)=>result.group===group&&<button key={result.id} id={`command-${index}`} role="option" aria-selected={active===index} onMouseMove={()=>setActive(index)} onClick={()=>select(result)}>{result.cover?<Cover src={result.cover} title={result.title}/>:<Icon name={result.icon??'search'}/>}<span><strong>{result.title}</strong><small>{result.detail}</small></span><kbd>↵</kbd></button>)}</section>)}{!results.length&&<p>No matches. Try an artist, album, song, or command.</p>}</div>}
 <footer><strong>Search MusicOS</strong><span>↑↓ Move · ↵ Select · Esc Close</span></footer></dialog>;
}
