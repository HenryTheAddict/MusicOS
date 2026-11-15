import {createContext,useContext,useEffect,useRef,useState,type ReactNode} from 'react';
import {call,errorText} from './bridge';
import {Cover,Icon,duration} from './components';
import type {Track} from './types';
import './songPreview.css';
type Preview={track_id:number;position:number;duration:number;preparing:boolean};
type Controls={trackId?:number;pending:boolean;toggle:(track:Track)=>void};
const Context=createContext<Controls>({pending:false,toggle:()=>{}});
export function useSongPreview(){return useContext(Context);}
export function PreviewButton({track}:{track:Track}){
 const value=useContext(Context),selected=value.trackId===track.id;
 return <button className="song-preview-action" aria-label={`${selected?'Stop preview of':'Preview'} ${track.title}`} title={selected?'Stop preview':'Preview 30 seconds'} aria-pressed={selected} disabled={!track.available||value.pending&&!selected} onClick={()=>value.toggle(track)}><Icon name={selected?'controlClose':'play'}/></button>;
}
export function SongPreviewProvider({children,active=true}:{children:ReactNode;active?:boolean}){
 const [track,setTrack]=useState<Track|null>(null),[preview,setPreview]=useState<Preview|null>(null),[pending,setPending]=useState(false),[error,setError]=useState('');
 const intent=useRef(0),alive=useRef(true);
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;intent.current++;void call('audition',{action:'stop'}).catch(()=>{});};},[]);
 async function stop(){intent.current++;setTrack(null);setPreview(null);setPending(false);await call('audition',{action:'stop'}).catch(e=>{if(alive.current)setError(errorText(e));});}
 useEffect(()=>{if(!active&&track)void stop();},[active]);
 async function toggle(next:Track){if(track?.id===next.id){await stop();return;}const token=++intent.current;setTrack(next);setPreview(null);setPending(true);setError('');try{await call('audition',{action:'start',trackId:next.id});}catch(e){if(alive.current&&token===intent.current){setError(errorText(e));setTrack(null);setPending(false);}}}
 useEffect(()=>{if(!track)return;const token=intent.current,began=performance.now();let timer=0,seen=false,cancelled=false;
  const poll=async()=>{try{const result=await call('audition',{action:'status'});if(cancelled||token!==intent.current)return;if(result.preview?.track_id===track.id){seen=true;setPreview(result.preview);setPending(result.preview.preparing);}else if(seen||performance.now()-began>5000){setTrack(null);setPreview(null);setPending(false);if(!seen)setError(result.error??'This song couldn’t be previewed.');return;}}catch(e){if(!cancelled&&token===intent.current){setError(errorText(e));setTrack(null);setPending(false);}return;}if(!cancelled)timer=window.setTimeout(poll,160);};void poll();return()=>{cancelled=true;clearTimeout(timer);};
 },[track]);
 return <Context.Provider value={{trackId:track?.id,pending,toggle:next=>void toggle(next)}}>{children}{track&&<aside className="song-preview-tray" aria-label="Song preview"><Cover src={track.cover_art_path} trackId={track.id} title={track.album}/><div><strong>{track.title}</strong><small>{pending?'Preparing preview…':`Preview · ${track.artist}`}</small><progress max={preview?.duration||30} value={preview?.position??0}/></div><span>{duration(Math.max(0,(preview?.duration??30)-(preview?.position??0)))}</span><button aria-label="Stop song preview" onClick={()=>void stop()}><Icon name="controlClose"/></button></aside>}{error&&<div className="song-preview-error" role="alert">{error}<button aria-label="Dismiss preview error" onClick={()=>setError('')}><Icon name="controlClose"/></button></div>}</Context.Provider>;
}
