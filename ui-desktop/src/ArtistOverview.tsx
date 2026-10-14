import { useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { Cover, collectionDuration } from "./components";
import {openArtwork} from './ArtworkViewer';
import {useDepthArtwork} from "./depthArtwork";
import {useArtistPointer} from "./portraitMotion";
import type { Album, Artist, ArtistInfo, Track } from "./types";
export default function ArtistOverview({artist,albums,tracks,info,fallback,actions,onProvider,scrollRef,depth}:{scrollRef:RefObject<HTMLElement|null>;depth:boolean;artist:Artist;albums:Album[];tracks:Track[];info:ArtistInfo|null;fallback?:Album;actions:ReactNode;onProvider:(url:string)=>void}) {
  const stage=useRef<HTMLDivElement>(null);useArtistPointer(stage);useDepthArtwork(stage,scrollRef,depth);
  const suppressRestoredFocus=useRef(false);
  const [hovered,setHovered]=useState(false);
  const [focused,setFocused]=useState(false);
  const [dismissed,setDismissed]=useState(false);
  const revealed=!dismissed&&(hovered||focused);
  const [expanded,setExpanded]=useState(false);
  useLayoutEffect(()=>{setHovered(false);setFocused(false);setDismissed(false);setExpanded(false);},[artist.id]);
  const genres=[...new Set(tracks.map(t=>t.genre).filter(g=>g&&g!=="Unknown genre"))].slice(0,2);
  const years=albums.map(a=>a.year).filter(Boolean).sort((a,b)=>a-b);
  const biography=info?.biography;
  const excerpt=biography && !expanded && biography.length>220 ? `${biography.slice(0,220).replace(/\s+\S*$/,"")}…` : biography;
  const facts=[...genres,`${albums.length} ${albums.length===1?"album":"albums"}`,`${artist.track_ids.length} ${artist.track_ids.length===1?"song":"songs"}`,...(years.length?[years[0]===years.at(-1)?String(years[0]):`${years[0]}–${years.at(-1)}`]:[])];
  return <header className={`artist-hero ${artist.name.length<12?"short-artist-name":""}`}>
    <h1>{artist.name}</h1>
    <section data-expanded={expanded || undefined} className="artist-about" aria-label={`About ${artist.name}`}>
      <h2>About</h2><div className="artist-biography"><p>{excerpt ?? (info?.lookup_status==='loading'?"Finding artist information…":"Biography unavailable.")}</p>
      {biography && biography.length>220 && <button className="text-link" aria-expanded={expanded} onClick={()=>setExpanded(!expanded)}>{expanded?"Read less":"Read more"}</button>}
      {info?.source_url && <button className="text-link" onClick={()=>onProvider(info.source_url!)}>Wikipedia ↗</button>}</div>
    </section>
    <div ref={stage} className="artist-portrait-stage" data-revealed={revealed || undefined} data-dismissed={dismissed || undefined} onMouseEnter={()=>{setHovered(true);setDismissed(false);}} onMouseLeave={()=>{setHovered(false);setDismissed(false);}} onFocus={e=>{if(suppressRestoredFocus.current){suppressRestoredFocus.current=false;setFocused(false);return;}if(e.target.matches(":focus-visible")){setFocused(true);setDismissed(false);}}} onBlur={e=>{if(!e.currentTarget.contains(e.relatedTarget))setFocused(false);}}>
      <button className="artist-portrait-button" onClick={e=>{suppressRestoredFocus.current=true;setFocused(false);setDismissed(true);openArtwork(e.currentTarget,artist.name);}} aria-label={`Expand ${artist.name} portrait`} aria-expanded={revealed} aria-controls="artist-portrait-notes" onKeyDown={e=>{if(e.key==="Escape"){setDismissed(true);}}}>
        <Cover src={info?.portrait??artist.portrait} title={artist.name} portrait />
      </button>
      <div className="portrait-notes" id="artist-portrait-notes" aria-label="Artist attributes">
        {facts.slice(0,5).map((fact,index)=><span className="portrait-fact" key={fact} style={{"--note-index":index} as React.CSSProperties}>{fact}</span>)}
        {info?.portrait_source_url && <button className="portrait-credit-link" onClick={()=>onProvider(info.portrait_source_url!)}>Photo: {info.portrait_credit?.replace(/https?:\/\/\S+/g,"Wikimedia Commons") || "Wikimedia Commons"}<small>{info.portrait_license || "View license at source"} ↗</small></button>}
      </div>
    </div>
    <div className="artist-statistics"><span>In your library</span><strong>{albums.length} {albums.length===1?"album":"albums"}</strong><span>{artist.track_ids.length} {artist.track_ids.length===1?"song":"songs"}</span><span>{collectionDuration(tracks.reduce((s,t)=>s+t.duration,0))}</span>{actions}</div>
  </header>;
}
