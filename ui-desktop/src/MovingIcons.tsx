import {useLayoutEffect,useRef,type CSSProperties} from 'react';
import {AssetIcon} from './AssetIcon';
import {currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import type {AlbumPalette} from './visualizer';

const icons=[
 ['Album','Song','Playlist','Lossless','Sparklist'],
 ['hearted','SmartPlaylist','Album','Song','Playlist'],
 ['Song','Sparklist','Lossless','hearted','SmartPlaylist'],
];
const colors=['#eea7c1','#ffc897','#93dddc','#b4b9fa','#b8e3a0'];
const bands=[0,1,1,2,2];

export default function MovingIcons({trackId,playing,active,reduced,palette}:{trackId?:number;playing:boolean;active:boolean;reduced:boolean;palette?:AlbumPalette}){
 const host=useRef<HTMLDivElement>(null);
 useLayoutEffect(()=>{
  const scene=host.current;
  if(!scene||!active||!playing||reduced||trackId===undefined)return;
  const envelope=[0,0,0],target=[0,0,0];
  let frame:number|undefined,previous:number|undefined;
  function stop(){
   if(frame!==undefined)cancelAnimationFrame(frame);
   frame=previous=undefined;
   envelope.fill(0);
   for(let band=0;band<3;band++)scene!.style.removeProperty(`--moving-level-${band}`);
  }
  function paint(now:number){
   frame=undefined;
   const elapsed=previous===undefined?16:Math.min(64,now-previous);
   previous=now;
   let settling=false;
   for(let band=0;band<3;band++){
    const delta=target[band]-envelope[band];
    if(Math.abs(delta)>.002){
     envelope[band]+=delta*(1-Math.exp(-elapsed/(delta>0?35:130)));
     settling=true;
    }else envelope[band]=target[band];
    scene!.style.setProperty(`--moving-level-${band}`,envelope[band].toFixed(3));
   }
   if(settling)frame=requestAnimationFrame(paint);
   else previous=undefined;
  }
  function sync(){
   const clock=currentPlaybackClock();
   if(document.hidden||!clock.playing||clock.trackId!==trackId){stop();return;}
   for(let band=0;band<3;band++){
    const level=clock.levels[band]??0;
    target[band]=Number.isFinite(level)?Math.sqrt(Math.min(1,Math.max(0,level))):0;
   }
   if(frame===undefined&&target.some((level,band)=>level!==envelope[band]))frame=requestAnimationFrame(paint);
  }
  const unsubscribe=subscribePlaybackClock(sync);
  document.addEventListener('visibilitychange',sync);
  sync();
  return()=>{
   unsubscribe();
   document.removeEventListener('visibilitychange',sync);
   stop();
  };
 },[active,playing,reduced,trackId]);

 return <div className="moving-icons-scene" ref={host} aria-hidden="true">
  <div className="moving-icons-grid">
   {icons.map((row,y)=>row.map((name,x)=>{
    const band=bands[x],swatch=palette?.[band],color=colors[x];
    return <span key={`${y}-${x}`} className="moving-icons-cell" style={{
     '--moving-color':swatch?`color-mix(in srgb,rgb(${swatch.map(channel=>Math.round(channel*255)).join(' ')}) 65%,${color})`:color,
     '--moving-level':`var(--moving-level-${band},0)`,
     '--moving-lift':y===1?'-.9rem':'-1.65rem',
     '--moving-tilt':(x+y)%2?'12deg':'-12deg',
    } as CSSProperties}><AssetIcon name={name} motion={false}/></span>;
   }))}
  </div>
 </div>;
}
