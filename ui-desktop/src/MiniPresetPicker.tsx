import {useMemo,useState,type CSSProperties} from 'react';
import MiniPlayer from './MiniPlayer';
import type {Playback,Preferences} from './types';
const presets={standard:{label:'Standard',dimensions:'244 × 320',description:'Artwork above the controls.'},compact:{label:'Compact',dimensions:'220 × 120',description:'Small artwork and controls in one row.'},wide:{label:'Wide',dimensions:'320 × 120',description:'More room for song details, with controls in one row.'}} as const;
const noop=()=>{};const action=async()=>{};
export default function MiniPresetPicker({playback,cover,value,onChange,theme}:{theme?:CSSProperties;playback:Playback;cover?:string|null;value:NonNullable<Preferences['mini_player_density']>;onChange:(density:NonNullable<Preferences['mini_player_density']>)=>void}){
 const [view,setView]=useState<'tall'|'queue'>('tall');
 // A still preview reuses the player composition without a playback clock or jobs.
 const snapshot=useMemo(()=>({...playback,is_playing:false,position:playback.track?playback.track.duration*.3:0,queue_index:null,queue:playback.queue.slice(playback.queue_index===null?0:playback.queue_index+1).slice(0,4)}),[playback.track?.id,cover]);
 const preset=presets[value];
 return <article className="settings-tile mini-presets">
  <h2>Mini Player</h2>
  <div className="mini-preset-layout">
   <div className="mini-preset-controls">
    <div className="size-options mini-preset-options" role="group" aria-label="Mini Player size">
     {(['standard','compact','wide'] as const).map(density=><button key={density} aria-pressed={value===density} onClick={()=>onChange(density)}>{presets[density].label}</button>)}
    </div>
    <p className="mini-preset-description">{preset.description}<small>{preset.dimensions} minimum</small></p>
    <div className="mini-preview-choice"><span>Preview</span><div className="size-options mini-preview-mode" role="group" aria-label="Preview Mini Player view"><button aria-pressed={view==='tall'} onClick={()=>setView('tall')}>Artwork</button><button aria-pressed={view==='queue'} onClick={()=>setView('queue')}>Up next</button></div></div>
   </div>
   <div className="mini-preview" style={theme} aria-label={`${preset.label} Mini Player preview`} aria-hidden="true" inert data-density={value}><MiniPlayer preview density={value} player={snapshot} cover={cover} mode={view} onMode={noop} onAction={action} onFavorite={noop} onAlbum={noop} onQueue={noop} onMenuResize={action} pinned={false} onPin={noop} onWindowError={noop} items={[]}/></div>
  </div>
 </article>;
}
