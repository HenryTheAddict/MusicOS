import {visualizerCatalog,type VisualizerPreferences} from './visualizer';
import ThemedChoice from './ThemedChoice';
import ThemedSwitch from './ThemedSwitch';
export default function VisualizerOptions({value,onChange,fallback=false,picker=true}:{value:VisualizerPreferences;onChange:(p:VisualizerPreferences)=>void;fallback?:boolean;picker?:boolean}){
 const update=(patch:Partial<VisualizerPreferences>)=>onChange({...value,...patch});
 return <section className="visualizer-options" aria-label="Visualizer options">
  {picker&&<div className="settings-group-row"><span>Effect</span><ThemedChoice label="Visualizer effect" value={value.preset} onChange={preset=>update({preset})} options={visualizerCatalog.map(p=>({value:p.id,label:p.name}))}/></div>}
  <div className="settings-group-row"><span>Preset changes</span><ThemedChoice label="Preset changes" value={value.rotation} onChange={rotation=>update({rotation:rotation as VisualizerPreferences['rotation']})} options={[{value:'manual',label:'Manual'},{value:'each_song',label:'Each song'},{value:'timed',label:'Timed'},{value:'beats',label:'Beats'}]}/></div>
  {value.rotation==='timed'&&<div className="settings-group-row"><span>Interval</span><ThemedChoice label="Timed interval" value={String(value.timed_seconds)} onChange={v=>update({timed_seconds:Number(v) as 15|30|60})} options={[15,30,60].map(v=>({value:String(v),label:`${v} seconds`}))}/></div>}
  {value.rotation==='beats'&&<><div className="settings-group-row"><span>Interval</span><ThemedChoice label="Beat interval" value={String(value.beat_interval)} onChange={v=>update({beat_interval:Number(v) as 16|32|64})} options={[16,32,64].map(v=>({value:String(v),label:`${v} beats`}))}/></div>{fallback&&<p className="visualizer-fallback" role="status">No reliable beat timing yet. Changing every 30 seconds.</p>}</>}
  <div className="settings-group-row"><ThemedSwitch label="Album colors" checked={!!value.album_colors} onChange={album_colors=>update({album_colors})}/></div>
  <div className="settings-group-row"><ThemedSwitch label="Beat Sync" checked={value.beat_sync} onChange={beat_sync=>update({beat_sync})}/></div>
  <div className="settings-group-row"><ThemedSwitch label="Visualizer lyrics" checked={value.lyrics} onChange={lyrics=>update({lyrics})}/></div>
 </section>;
}
