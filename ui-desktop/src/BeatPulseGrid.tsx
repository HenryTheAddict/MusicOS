import {useEffect,useRef,useState,type CSSProperties} from 'react';
import {AssetIcon} from './AssetIcon';
import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import type {AlbumPalette} from './visualizer';

export function beatPosition(position:number,bpm?:number,offset=0){
 return Number.isFinite(position)&&Number.isFinite(offset)&&bpm!==undefined&&Number.isFinite(bpm)&&bpm>=20&&bpm<=400?(position-offset)*bpm/60+.1:null;
}
export function ripplePulse(beat:number|null,distance:number){
 if(beat===null)return 0;
 const phase=((beat-distance*.16)%1+1)%1;
 return Math.max(0,1-phase/.65)**2;
}
export function downbeatGlyph(beat:number|null,index:number){
 if(beat===null||beat<0)return 0;
 const bar=Math.floor(beat/4),group=index%7;
 return Math.max(0,Math.floor((bar+6-group)/7))%3;
}
export function gridSize(width:number,height:number){return {columns:Math.max(3,Math.round(width/112)),rows:Math.max(3,Math.round(height/112))};}
const glyphs=['Song','hearted','play'];
export default function BeatPulseGrid({bpm,offset,trackId,playing,active,reduced,palette}:{bpm?:number;offset?:number;trackId?:number;playing:boolean;active:boolean;reduced:boolean;palette?:AlbumPalette}){
 const host=useRef<HTMLDivElement>(null),[size,setSize]=useState({columns:3,rows:3});
 useEffect(()=>{const observer=new ResizeObserver(([entry])=>setSize(gridSize(entry.contentRect.width,entry.contentRect.height)));observer.observe(host.current!);return()=>observer.disconnect();},[]);
 useEffect(()=>{
  const cells=Array.from(host.current!.children) as HTMLElement[];
  let frame:number|undefined;
  const paint=(animated:boolean)=>{
   const clock=currentPlaybackClock(),beat=animated&&clock.trackId===trackId?beatPosition(clockPosition(clock),bpm,offset):null;
   const levels=clock.levels.filter(Number.isFinite),amplitude=levels.length?Math.max(.45,Math.min(1,Math.max(...levels))):.7;
   cells.forEach((cell,index)=>{
    const x=index%size.columns-(size.columns-1)/2,y=Math.floor(index/size.columns)-(size.rows-1)/2,distance=Math.hypot(x,y),pulse=ripplePulse(beat,distance)*amplitude;
    cell.style.transform=`translate(${x/(distance||1)*pulse*12}px,${y/(distance||1)*pulse*12}px) scale(${1+pulse*.32}) rotate(${Math.sin(index*2.4)*8}deg)`;
    cell.style.opacity=String(.6+pulse*.4);
    const selected=downbeatGlyph(beat,index);
    Array.from(cell.children).forEach((icon,i)=>(icon as HTMLElement).style.opacity=i===selected?'1':'0');
   });
  };
  const stop=()=>{if(frame!==undefined)cancelAnimationFrame(frame);frame=undefined;};
  const tick=()=>{paint(true);frame=requestAnimationFrame(tick);};
  const sync=()=>{stop();const clock=currentPlaybackClock();if(active&&playing&&!reduced&&!document.hidden&&clock.playing&&clock.trackId===trackId){frame=requestAnimationFrame(tick);}else paint(false);};
  sync();const off=subscribePlaybackClock(sync);document.addEventListener('visibilitychange',sync);
  return()=>{stop();off();document.removeEventListener('visibilitychange',sync);};
 },[size,bpm,offset,trackId,playing,active,reduced]);
 return <div ref={host} className="beat-pulse-grid" aria-hidden="true" style={{gridTemplateColumns:`repeat(${size.columns},1fr)`,gridTemplateRows:`repeat(${size.rows},1fr)`} as CSSProperties}>{Array.from({length:size.columns*size.rows},(_,index)=><div className="beat-pulse-cell" key={index} style={palette?.length?{'--beat-color':`rgb(${palette[index%palette.length].map(c=>Math.round(c*255)).join(' ')})`} as CSSProperties:undefined}>{glyphs.map(name=><AssetIcon key={name} name={name} motion={false}/>)}</div>)}</div>;
}
