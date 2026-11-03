import {useEffect,useRef,useState,useId,type ComponentProps} from 'react';
import {listen} from '@tauri-apps/api/event';
import ExpandedPlayer from './ExpandedPlayer';
import {call,native,errorText} from './bridge';
import {beatOffsets} from './heartVisibility';
import {defaultVisualizer,type VisualizerSettings,type VisualizerStatus} from './visualizer';
import {useVisualizerAlbumPalette} from './visualizerAlbumPalette';
import VisualizerPicker from './VisualizerPicker';
import VisualizerSettingsPanel from './VisualizerSettings';
import type {Preferences} from './types';
import './visualizer.css';
let windowOperation=Promise.resolve<unknown>(undefined);
// Keep a reloaded webview newer than its previous in-flight native requests.
let nextRequestId=Date.now()*1000;
function serializeWindow(work:()=>Promise<unknown>){const next=windowOperation.then(work);windowOperation=next.catch(()=>{});return next;}
export default function VisualizerPlayer({preferences,onPreferences,...props}:{preferences:Preferences;onPreferences:(p:Preferences)=>void}&ComponentProps<typeof ExpandedPlayer>){
 const [reduced,setReduced]=useState(()=>matchMedia('(prefers-reduced-motion: reduce)').matches);
 const prefs=preferences.visualizer??defaultVisualizer,settings=useRef<VisualizerSettings>(null!),[ready,setReady]=useState(!native),[status,setStatus]=useState<VisualizerStatus|null>(null),[error,setError]=useState(''),[attempt,setAttempt]=useState(0);
 const latest=useRef({preferences,onPreferences});latest.current={preferences,onPreferences};
 const id=props.player.track?.id,offset=id===undefined?undefined:beatOffsets.get(id);
 const colorFilter=useId();
 const albumPalette=useVisualizerAlbumPalette(!!prefs.album_colors,id,props.cover);
 const beat={trackId:id,bpm:props.bpm,offset,confidence:offset===undefined?0:0.5};
 const configIdentity=useRef(''),requestId=useRef(0),identity=JSON.stringify([prefs,albumPalette,beat,reduced,props.player.is_playing,props.player.listening?.speed,props.player.queue_entry_ids?.[props.player.queue_index??-1]]);
 if(configIdentity.current!==identity){configIdentity.current=identity;requestId.current=++nextRequestId;}
 settings.current={requestId:requestId.current,preferences:prefs,albumPalette,beat,reduced};
 useEffect(()=>{
  let alive=true,off:(()=>void)|undefined,previousFull:boolean|undefined;
  const restore=async()=>{if(native){await call('visualizer',{request:{action:'close'}}).catch(()=>{});if(previousFull!==undefined){const {getCurrentWindow}=await import('@tauri-apps/api/window');await getCurrentWindow().setFullscreen(previousFull).catch(()=>{});}}else if(document.fullscreenElement)await document.exitFullscreen().catch(()=>{});delete document.documentElement.dataset.nativeVisualizer;};
  const open=async()=>{
   try{
    if(native){const {getCurrentWindow}=await import('@tauri-apps/api/window');const w=getCurrentWindow();previousFull=await w.isFullscreen();if(!alive)return;await w.setFullscreen(true);if(!alive)return;
     const stop=await listen<VisualizerStatus>('musicos-visualizer-status',({payload})=>{if(!alive||payload.requestId!==settings.current.requestId)return;setStatus(payload);if(payload.error)setError(payload.error);if(payload.preset&&payload.preset!==(latest.current.preferences.visualizer??defaultVisualizer).preset){const p=latest.current.preferences;latest.current.onPreferences({...p,visualizer:{...(p.visualizer??defaultVisualizer),preset:payload.preset}});}});if(!alive){stop();return;}off=stop;
     document.documentElement.dataset.nativeVisualizer='true';const opened=await call('visualizer',{request:{action:'open',settings:settings.current}});if(alive&&opened.requestId===settings.current.requestId)setStatus(opened);
    }else{throw Error('Native Metal visualizers are available in the macOS desktop app.');}
    if(alive)setReady(true);
   }catch(e){if(alive){setError(errorText(e));setReady(true);}}
  };setError('');setReady(!native);void serializeWindow(open);
  return()=>{alive=false;off?.();void serializeWindow(restore);};
 },[attempt]);
 useEffect(()=>{if(!native||!ready)return;let alive=true;const revision=settings.current.requestId;void call('visualizer',{request:{action:'refresh',settings:settings.current}}).catch(e=>{if(alive&&revision===settings.current.requestId)setError(errorText(e));});return()=>{alive=false;};},[ready,prefs,albumPalette,id,offset,props.bpm,props.player.is_playing,props.player.listening?.speed,props.player.queue_entry_ids?.[props.player.queue_index??-1]]);
 useEffect(()=>{const media=matchMedia('(prefers-reduced-motion: reduce)'),update=()=>{setReduced(media.matches);settings.current={...settings.current,reduced:media.matches};if(native&&ready)void call('visualizer',{request:{action:'refresh',settings:settings.current}}).catch(()=>{});};media.addEventListener('change',update);return()=>media.removeEventListener('change',update);},[ready]);
 const view=props.view;
 const stills=import.meta.glob<string>('../../assets/visualizers/thumbnails/*.png',{query:'?url',import:'default',eager:true});
 return <ExpandedPlayer {...props} view={view} onView={next=>{props.onView(next);onPreferences({...preferences,visualizer:{...prefs,lyrics:next==='lyrics'}});}} visualizerMode fullscreenManaged visualizerControls={<><VisualizerPicker value={status?.preset||prefs.preset} onChange={preset=>onPreferences({...preferences,visualizer:{...prefs,preset}})}/><VisualizerSettingsPanel value={prefs} fallback={!!status?.beatFallback} onChange={visualizer=>{onPreferences({...preferences,visualizer});if(visualizer.lyrics!==prefs.lyrics)props.onView(visualizer.lyrics?'lyrics':'playing');}}/></>} visualizerStatus={<>{reduced&&<>{albumPalette&&<svg width="0" height="0" aria-hidden="true"><filter id={colorFilter} colorInterpolationFilters="sRGB"><feColorMatrix type="saturate" values="0"/><feComponentTransfer>{(['R','G','B'] as const).map((channel,index)=>{const Component=(`feFunc${channel}`) as 'feFuncR';return <Component key={channel} type="table" tableValues={[0,...albumPalette.map(color=>color[index]),1].join(' ')}/>;})}</feComponentTransfer></filter></svg>}<img className="visualizer-still" style={albumPalette?{filter:`url(#${colorFilter})`}:undefined} src={stills[`../../assets/visualizers/thumbnails/${status?.preset||prefs.preset}.png`]} alt="" aria-hidden="true"/></>}{error?<div className="visualizer-unavailable" role="alert"><strong>Visualizer unavailable</strong><p>{error}</p><button className="soft-action" onClick={()=>{void call('visualizer',{request:{action:'close'}}).finally(()=>setAttempt(v=>v+1));}}>Retry</button></div>:!ready||status?.phase==='starting'?<div className="visualizer-starting" role="status">Preparing visualizer…</div>:null}</>} onToggleFullscreen={props.onClose}/>;
}
