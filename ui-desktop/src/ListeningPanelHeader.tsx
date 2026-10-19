import type {ReactNode} from 'react';
import {Cover} from './components';
import type {Track} from './types';

export default function ListeningPanelHeader({title,closeLabel,onClose,track,detail}:{title:string;closeLabel:string;onClose:()=>void;track:Track|null;detail?:ReactNode}){
 return <>
  <header className="listening-panel-header"><h2>{title}</h2><button className="listening-panel-close" aria-label={closeLabel} onClick={onClose}>×</button></header>
  <div className="listening-panel-current"><Cover songArtwork src={track?.cover_art_path} trackId={track?.id} title={track?.album??'No music playing'}/><div><strong>{track?.title??'Nothing playing'}</strong><small>{detail??(track?`${track.artist} · ${track.album}`:'Choose music in Library')}</small></div></div>
 </>;
}
