import {useRef,type RefObject} from 'react';
import {Cover} from './components';
import {useDepthArtwork} from './depthArtwork';
import {openArtwork} from './ArtworkViewer';
import type {Album} from './types';
export default function AlbumArtwork({album,scrollRef,depth,motion}:{album:Album;scrollRef:RefObject<HTMLElement|null>;depth:boolean;motion:boolean}){
 const stage=useRef<HTMLDivElement>(null);
 useDepthArtwork(stage,scrollRef,depth);
 return <div ref={stage} className="album-artwork-stage"><button className="album-artwork" aria-label={`Expand ${album.title} artwork`} onClick={e=>openArtwork(e.currentTarget,album.title)}>
  <Cover src={album.cover} trackId={album.track_ids[0]} title={album.title}/>
 </button></div>;
}
