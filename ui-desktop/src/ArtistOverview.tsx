import { useLayoutEffect, useRef, useState, useId, type ReactNode } from "react";
import SecondaryAction from './SecondaryAction';
import { Cover, collectionDuration } from "./components";
import {animateOwned} from './presentationMotion';
import {openArtwork} from './ArtworkViewer';
import type { Album, Artist, ArtistInfo, Track } from "./types";
const expandedArtists=new Map<string,boolean>();
export default function ArtistOverview({artist,albums,tracks,info,fallback,actions,onProvider}:{artist:Artist;albums:Album[];tracks:Track[];info:ArtistInfo|null;fallback?:Album;actions:ReactNode;onProvider:(url:string)=>void}) {
  const root=useRef<HTMLElement>(null);
  const previousHeight=useRef<number|null>(null);
  const [expanded,setExpanded]=useState(()=>expandedArtists.get(artist.id)??false);
  useLayoutEffect(()=>{setExpanded(expandedArtists.get(artist.id)??false);},[artist.id]);
  function expand(){
    previousHeight.current=root.current?.getBoundingClientRect().height??null;
    const next=!expanded;if(expandedArtists.size>=32&&!expandedArtists.has(artist.id))expandedArtists.delete(expandedArtists.keys().next().value!);expandedArtists.set(artist.id,next);setExpanded(next);
  }
  useLayoutEffect(()=>{
    const before=previousHeight.current,node=root.current;previousHeight.current=null;
    if(before===null||!node||matchMedia('(prefers-reduced-motion: reduce)').matches)return;
    const height=node.getBoundingClientRect().height;
    if(Math.abs(height-before)<1)return;
    // Only the containing height moves; text and the summary retain their geometry.
    const stop=animateOwned(node,'biography-height',[
      {height:`${before}px`,overflow:'clip'},
      {height:`${height}px`,overflow:'clip'},
    ],180);
    window.addEventListener('resize',stop,{once:true});
    window.addEventListener('scroll',stop,{capture:true,once:true});
    return()=>{stop();window.removeEventListener('resize',stop);window.removeEventListener('scroll',stop,true);};
  },[expanded]);
  const biography=info?.biography;
  const biographyRef=useRef<HTMLParagraphElement>(null),biographyId=useId();
  const [overflowing,setOverflowing]=useState(false);
  useLayoutEffect(()=>{
    if(expanded)return;
    const paragraph=biographyRef.current;if(!paragraph)return;
    const measure=()=>setOverflowing(paragraph.scrollHeight>paragraph.clientHeight+1);
    const observer=new ResizeObserver(measure);observer.observe(paragraph);measure();
    return()=>observer.disconnect();
  },[biography,expanded]);
  return <header ref={root} data-biography-expanded={expanded||undefined} className="artist-hero">
    <h1>{artist.name.split(/(\s+)/).map((part,index)=>/^\s+$/.test(part)?part:<span className="artist-name-part" key={index}>{part}</span>)}</h1>
    <section data-expanded={expanded || undefined} className="artist-about" aria-label={`About ${artist.name}`}>
      <div className="artist-about-heading"><h2>About</h2>{biography && (overflowing||expanded) && <SecondaryAction expanded={expanded} aria-controls={biographyId} aria-expanded={expanded} onClick={expand}>{expanded?"Read less":"Read more"}</SecondaryAction>}</div>
      <div className="artist-biography"><div id={biographyId}>{(expanded&&biography?biography.split(/\n\s*\n|\n/):[biography??(info?.lookup_status==='loading'?"Finding artist information…":"Biography unavailable.")]).map((paragraph,index)=><p key={index} ref={index===0?biographyRef:undefined}>{paragraph}</p>)}</div></div>
      <div className="artist-biography-actions">
      {info?.source_url && <button className="text-link" onClick={()=>onProvider(info.source_url!)}>Wikipedia ↗</button>}</div>
    </section>
    <div className="artist-summary">
    <div className="artist-portrait-stage">
      <button className="artist-portrait-button" onClick={e=>openArtwork(e.currentTarget,artist.name)} aria-label={`Expand ${artist.name} portrait`} aria-haspopup="dialog">
        <Cover src={info?.portrait??artist.portrait} title={artist.name} portrait />
      </button>
    </div>
    <div className="artist-statistics"><span>In your library</span><div className="artist-library-counts"><strong>{albums.length} {albums.length===1?"album":"albums"}</strong><span>{artist.track_ids.length} {artist.track_ids.length===1?"song":"songs"}</span></div><span>{collectionDuration(tracks.reduce((s,t)=>s+t.duration,0))}</span></div>
    <div className="artist-playback">{actions}</div>
    {info?.portrait_source_url && <details className="artist-photo-credit" key={artist.id}><summary>Photo credit</summary><p>{info.portrait_credit?.replace(/https?:\/\/\S+/g,"Wikimedia Commons") || "Wikimedia Commons"}</p><button className="text-link" onClick={()=>onProvider(info.portrait_source_url!)}>{info.portrait_license || "View license at source"} ↗</button></details>}
    </div>
  </header>;
}
