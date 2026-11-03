import {useEffect,useRef,useState} from 'react';
import type {Track} from './types';
import {Icon} from './components';

/** Acknowledged handoff feedback; never a speculative title or playback command. */
export default function SongChangeNotice({track,entryId,active}:{track:Track|null;entryId?:number;active:boolean}){
 const identity=`${track?.id??''}:${entryId??''}`,last=useRef(identity);
 const [notice,setNotice]=useState<Track|null>(null),[visible,setVisible]=useState(false);
 useEffect(()=>{
  const changed=last.current!==identity&&!!last.current.split(':')[0];last.current=identity;
  if(!active||!track||!changed){setVisible(false);setNotice(null);return;}
  setNotice(track);setVisible(true);
  const hide=setTimeout(()=>setVisible(false),2200),clear=setTimeout(()=>setNotice(null),2380);
  return()=>{clearTimeout(hide);clearTimeout(clear);};
 },[identity,active]);
 return <div className="song-change-notice" data-visible={visible&&active||undefined} role="status" aria-live="polite"><Icon name="Song"/><span><strong>{notice?.title}</strong><small>{notice?.artist}</small></span></div>;
}
