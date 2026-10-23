import { useLayoutEffect, useRef, useState, useId, type ReactNode } from "react";
import { Cover, collectionDuration } from "./components";
import {animateOwned} from './presentationMotion';
import {openArtwork} from './ArtworkViewer';
import type { Album, Artist, ArtistInfo, Track } from "./types";
export default function ArtistOverview({artist,albums,tracks,info,fallback,actions,onProvider}:{artist:Artist;albums:Album[];tracks:Track[];info:ArtistInfo|null;fallback?:Album;actions:ReactNode;onProvider:(url:string)=>void}) {
  const suppressRestoredFocus=useRef(false),root=useRef<HTMLElement>(null);
  const previousBounds=useRef<{height:number;boxes:Map<Element,DOMRect>}|null>(null);
  const [hovered,setHovered]=useState(false);
  const [focused,setFocused]=useState(false);
  const [dismissed,setDismissed]=useState(false);
  const revealed=!dismissed&&(hovered||focused);
  const [expanded,setExpanded]=useState(false);
  useLayoutEffect(()=>{setHovered(false);setFocused(false);setDismissed(false);setExpanded(false);},[artist.id]);
  function expand(){const node=root.current;if(node)previousBounds.current={height:node.getBoundingClientRect().height,boxes:new Map([...node.children].map(child=>[child,child.getBoundingClientRect()]))};setExpanded(value=>!value);}
  useLayoutEffect(()=>{const before=previousBounds.current,node=root.current;previousBounds.current=null;if(!before||!node||matchMedia('(prefers-reduced-motion: reduce)').matches)return;const stops:(()=>void)[]=[],height=node.getBoundingClientRect().height;for(const child of node.children){const from=before.boxes.get(child),to=child.getBoundingClientRect();if(from)stops.push(animateOwned(child as HTMLElement,'biography-layout',[{opacity:.7,transform:`translate(${from.left-to.left}px,${from.top-to.top}px)`},{opacity:1,transform:'none'}],180));}if(Math.abs(height-before.height)>1)stops.push(animateOwned(node,'biography-height',[{height:`${before.height}px`,overflow:'clip'},{height:`${height}px`,overflow:'clip'}],180));return()=>stops.forEach(stop=>stop());},[expanded]);
  const genres=[...new Set(tracks.map(t=>t.genre).filter(g=>g&&g!=="Unknown genre"))].slice(0,2);
  const years=albums.map(a=>a.year).filter(Boolean).sort((a,b)=>a-b);
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
  const facts=[...genres,`${albums.length} ${albums.length===1?"album":"albums"}`,`${artist.track_ids.length} ${artist.track_ids.length===1?"song":"songs"}`,...(years.length?[years[0]===years.at(-1)?String(years[0]):`${years[0]}–${years.at(-1)}`]:[])];
  return <header ref={root} data-biography-expanded={expanded||undefined} className={`artist-hero ${artist.name.length<12?"short-artist-name":""}`}>
    <h1>{artist.name.split(/(\s+)/).map((part,index)=>/^\s+$/.test(part)?part:<span className="artist-name-part" key={index}>{part}</span>)}</h1>
    <section data-expanded={expanded || undefined} className="artist-about" aria-label={`About ${artist.name}`}>
      <h2>About</h2><div className="artist-biography"><div id={biographyId}>{(expanded&&biography?biography.split(/\n\s*\n|\n/):[biography??(info?.lookup_status==='loading'?"Finding artist information…":"Biography unavailable.")]).map((paragraph,index)=><p key={index} ref={index===0?biographyRef:undefined}>{paragraph}</p>)}</div></div>
      <div className="artist-biography-actions">{biography && (overflowing||expanded) && <button className="text-link" aria-controls={biographyId} aria-expanded={expanded} onClick={expand}>{expanded?"Read less":"Read more"}</button>}
      {info?.source_url && <button className="text-link" onClick={()=>onProvider(info.source_url!)}>Wikipedia ↗</button>}</div>
    </section>
    <div className="artist-portrait-stage" data-revealed={revealed || undefined} data-dismissed={dismissed || undefined} onMouseEnter={()=>{setHovered(true);setDismissed(false);}} onMouseLeave={()=>{setHovered(false);setDismissed(false);}} onFocus={e=>{if(suppressRestoredFocus.current){suppressRestoredFocus.current=false;setFocused(false);return;}if(e.target.matches(":focus-visible")){setFocused(true);setDismissed(false);}}} onBlur={e=>{if(!e.currentTarget.contains(e.relatedTarget))setFocused(false);}}>
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
