import {useEffect,useRef,useState,type CSSProperties} from 'react';
import {AssetIcon} from './AssetIcon';
import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import type {AlbumPalette} from './visualizer';
import {morphSvg} from './svgMorph';

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
export function gridSize(width:number,height:number){return {columns:Math.max(1,Math.min(16,Math.floor(width/104))),rows:Math.max(1,Math.min(10,Math.floor(height/104)))};}
const glyphs=['Song','hearted','play','Album','Sparklist','Lossless','Playlist'];
const colors=['#fc78a9','#61d8d4','#ffc470','#9c9aff','#7bd8a0','#8dbbff','#ff916e'];
type BeatPulseGridProps = {
  beatSync?: boolean;
  bpm?: number;
  offset?: number;
  trackId?: number;
  playing: boolean;
  active: boolean;
  reduced: boolean;
  palette?: AlbumPalette;
};

export default function BeatPulseGrid({
  beatSync = true, bpm, offset, trackId, playing, active, reduced, palette,
}: BeatPulseGridProps) {
 const host = useRef<HTMLDivElement>(null);
 const [size, setSize] = useState({ columns: 3, rows: 3 });

 useEffect(() => {
  const grid = host.current;
  if (!grid) return;
  const observer = new ResizeObserver(([entry]) => {
   const next = gridSize(entry.contentRect.width, entry.contentRect.height);
   setSize(current => current.columns === next.columns && current.rows === next.rows ? current : next);
  });
  observer.observe(grid);
  return () => observer.disconnect();
 }, []);

 useEffect(() => {
  const grid = host.current;
  if (!grid) return;
  const radius=Math.max(1,Math.hypot((size.columns-1)/2,(size.rows-1)/2));
  const cells=Array.from(grid.children,(node,index)=>{
   const x=index%size.columns-(size.columns-1)/2;
   const y=Math.floor(index/size.columns)-(size.rows-1)/2;
   const distance=Math.hypot(x,y);
   const icons=Array.from(node.querySelectorAll<HTMLElement>('.asset-icon'));
   const morph=node.querySelector<HTMLElement>('.beat-pulse-morph')!;
   icons.forEach(icon=>{icon.style.opacity='0';delete icon.dataset.beatSelected;});
   const depth=1-distance/radius;
   const wave=icons.find(icon=>icon.dataset.icon==='Lossless')?.querySelector('path');
   return {node:node as HTMLElement,icons,morph,wave,depth,pulse:-1,drive:-1,x:x/(distance||1),y:y/(distance||1),distance,rotation:Math.sin(index*2.4)*8,selected:-1};
  });
  let frame:number|undefined;
  let previousFrame:number|undefined;
  const envelope=[0,0,0];
  const morphs=new Map<HTMLElement,()=>void>();
  function clearMorph(host:HTMLElement) {
   morphs.get(host)?.();
   morphs.delete(host);
   delete host.dataset.morphing;
   host.replaceChildren();
  }
  const paint=(animated:boolean,now=0)=>{
   const clock=currentPlaybackClock();
   const beat=animated&&beatSync?beatPosition(clockPosition(clock),bpm,offset):null;
   const elapsed=previousFrame===undefined?16:Math.max(0,Math.min(80,now-previousFrame));
   previousFrame=animated?now:undefined;
   for(let i=0;i<envelope.length;i++){
    const level=clock.levels[i]??0;
    const target=Number.isFinite(level)?Math.max(0,Math.min(1,level)):0;
    const response=target>envelope[i]?38:145;
    envelope[i]=animated?envelope[i]+(target-envelope[i])*(1-Math.exp(-elapsed/response)):0;
   }
   const energy=Math.hypot(...envelope)/Math.sqrt(envelope.length);
   cells.forEach((cell,index)=>{
    const band=envelope[index%envelope.length];
    const strength=.6*energy+.4*band;
    const wave=beat===null?1:.24+.76*ripplePulse(beat,cell.distance);
    const pulse=Math.round(strength*wave*256)/256;
    if(pulse!==cell.pulse){
     const scale=.68+cell.depth*.32+pulse*.32;
     cell.node.style.transform=`translate(${cell.x*pulse*12}px,${cell.y*pulse*12}px) scale(${scale}) rotate(${cell.rotation}deg)`;
     cell.node.style.opacity=String(Math.min(1,.3+cell.depth*.31+pulse*.55));
     cell.pulse=pulse;
    }
    const selected=downbeatGlyph(beat,index);
    const name=cell.icons[selected].dataset.icon;
    const secondThump=name==='hearted'&&beat!==null ? .38*ripplePulse(beat-.18,cell.distance)*strength : 0;
    const drive=Math.round(Math.min(1,pulse+secondThump)*64)/64;
    if(drive!==cell.drive||selected!==cell.selected){
     cell.node.style.setProperty('--glyph-drive',String(drive));
     if(name==='Lossless'&&cell.wave){
      const bend=8+5*drive;
      cell.wave.setAttribute('d',`M4,16 C8,16 8,${16-bend} 12,${16-bend} C16,${16-bend} 16,${16+bend} 20,${16+bend} C24,${16+bend} 24,16 28,16`);
     }
     cell.drive=drive;
    }
    if(selected!==cell.selected){
     const previous=cell.icons[cell.selected];
     const next=cell.icons[selected];
     clearMorph(cell.morph);
     // Only a couple of icons morph at once. Other changes keep the lighter CSS reveal.
     if(animated&&morphs.size<2&&previous?.firstElementChild&&next.firstElementChild){
      try {
       const cancel=morphSvg(cell.morph,previous.innerHTML,next.innerHTML,260,()=>clearMorph(cell.morph));
       if(cell.morph.dataset.morphing==='true')morphs.set(cell.morph,cancel);
      } catch {
       clearMorph(cell.morph);
      }
     }
     if(previous){previous.style.opacity='0';delete previous.dataset.beatSelected;}
     next.style.opacity='1';
     next.dataset.beatSelected='true';
     cell.selected=selected;
    }
   });
  };
  function stop() {
   if (frame === undefined) return;
   cancelAnimationFrame(frame);
   frame = undefined;
  }

  function tick(now: number) {
   paint(true, now);
   frame = requestAnimationFrame(tick);
  }

  function sync() {
   const clock = currentPlaybackClock();
   const shouldAnimate = active && playing && !reduced && !document.hidden
    && clock.playing && clock.trackId === trackId;
   if (shouldAnimate) {
    if (frame === undefined) frame = requestAnimationFrame(tick);
   } else if (frame !== undefined) {
    stop();
    morphs.forEach((_,host)=>clearMorph(host));
    paint(false);
   }
  }

  paint(false);
  sync();
  const unsubscribe = subscribePlaybackClock(sync);
  document.addEventListener('visibilitychange', sync);
  return () => {
   stop();
   unsubscribe();
   morphs.forEach((_,host)=>clearMorph(host));
   document.removeEventListener('visibilitychange', sync);
  };
 }, [size, beatSync, bpm, offset, trackId, playing, active, reduced]);
 const gridStyle={gridTemplateColumns:`repeat(${size.columns},1fr)`,gridTemplateRows:`repeat(${size.rows},1fr)`};
 return <div className="beat-pulse-scene" aria-hidden="true">
  <div ref={host} className="beat-pulse-grid" style={gridStyle}>
   {Array.from({length:size.columns*size.rows},(_,index)=>{
    const swatch=palette?.length?palette[index%palette.length]:undefined;
    const accent=colors[index%colors.length];
    const color=swatch?`color-mix(in srgb,rgb(${swatch.map(c=>Math.round(c*255)).join(' ')}) 64%,${accent})`:accent;
    return <div className="beat-pulse-cell" key={index} style={{'--beat-color':color} as CSSProperties}>
     {[0,1,2].map(slot=>{
      const name=glyphs[(index*2+slot)%glyphs.length];
      return <AssetIcon key={name} name={name} motion={false}/>;
     })}
     <i className="icon beat-pulse-morph" />
    </div>;
   })}
  </div>
 </div>;
}
