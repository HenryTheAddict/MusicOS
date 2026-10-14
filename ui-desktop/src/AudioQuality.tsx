import {useEffect,useId,useLayoutEffect,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import {Icon} from './components';
import type {Track} from './types';
type Specs=Awaited<ReturnType<typeof getSpecs>>;
const requests=new Map<string,ReturnType<typeof getSpecs>>();
function getSpecs(trackId:number){return call('audio_specs',{trackId});}
export default function AudioQuality({tracks,compact=false}:{tracks:Track[];compact?:boolean}){
 const unique=useId().replace(/:/g,'');
 const tooltipId=`audio-specs-${compact?'player':'album'}-${unique}`;
 const [specs,setSpecs]=useState<Specs|null>(null),[error,setError]=useState(''),[loading,setLoading]=useState(false);
 const anchor=useRef<HTMLSpanElement>(null),tooltip=useRef<HTMLSpanElement>(null);
 const [hovered,setHovered]=useState(false),[focused,setFocused]=useState(false);
 const open=hovered||focused;
 useEffect(()=>{const clear=()=>{requests.clear();setSpecs(null);};window.addEventListener('musicos-audio-rescanned',clear);return()=>window.removeEventListener('musicos-audio-rescanned',clear);},[]);
 useLayoutEffect(()=>{
  const panel=tooltip.current,button=anchor.current;
  if(!panel||!button)return;
  if(!open){panel.hidePopover();return;}
  const showFrame=requestAnimationFrame(()=>{if(panel.isConnected){panel.showPopover();position();}});
  const position=()=>{
   const rect=button.getBoundingClientRect(),bounds=panel.getBoundingClientRect();
   const gap=6,edge=8;
   const below=rect.bottom+gap;
   const top=below+bounds.height<=window.innerHeight-edge?below:rect.top-gap-bounds.height;
   panel.style.left=`${Math.max(edge,Math.min(rect.right-bounds.width,window.innerWidth-bounds.width-edge))}px`;
   panel.style.top=`${Math.max(edge,Math.min(top,window.innerHeight-bounds.height-edge))}px`;
  };
  const close=()=>{setHovered(false);setFocused(false);};
  const dismiss=(event:KeyboardEvent)=>{if(event.key==='Escape')close();};
  const outside=(event:PointerEvent)=>{if(!button.contains(event.target as Node))close();};
  position();window.addEventListener('resize',position);window.addEventListener('scroll',position,true);window.addEventListener('keydown',dismiss);window.addEventListener('pointerdown',outside,true);window.addEventListener('blur',close);
  return()=>{cancelAnimationFrame(showFrame);window.removeEventListener('resize',position);window.removeEventListener('scroll',position,true);window.removeEventListener('keydown',dismiss);window.removeEventListener('pointerdown',outside,true);window.removeEventListener('blur',close);panel.hidePopover();};
 },[open,specs,error,loading]);
 const track=tracks[0];
 const signature=track?`${track.id}:${track.path}:${JSON.stringify(track.audio_quality)}`:'';
 const currentSignature=useRef(signature);currentSignature.current=signature;
 const formats=[...new Set(tracks.map(t=>t.path.split('.').at(-1)?.toUpperCase()??'Unknown'))];
 const quality=specs??track?.audio_quality;
 const known=tracks.map(t=>t.audio_quality).filter(Boolean);
 const label=tracks.length>1?(known.length<tracks.length?'Audio quality':known.every(q=>q!.lossless)?'Lossless':known.every(q=>!q!.lossless)?'Lossy':'Mixed quality'):quality?(quality.lossless?'Lossless':'Lossy'):'Audio quality';
 useEffect(()=>{setSpecs(null);if(track)void inspect();},[signature]);
 async function inspect(){if(!track)return;setLoading(true);setError('');try{let request=requests.get(signature);if(!request){request=getSpecs(track.id);requests.set(signature,request);request.catch(()=>requests.delete(signature));}const value=await request;if(currentSignature.current===signature)setSpecs(value);}catch(e){if(currentSignature.current===signature)setError(errorText(e));}finally{if(currentSignature.current===signature)setLoading(false);}}
 return <span ref={anchor} className="audio-quality" onMouseEnter={()=>{if(track){setHovered(true);void inspect();}}} onMouseLeave={()=>setHovered(false)}><button className={`quality ${compact?'compact-quality':''}`} aria-label={`Audio quality: ${label}. Open audio controls; hover for specifications`} onFocus={e=>{setFocused(e.currentTarget.matches(':focus-visible'));void inspect();}} onBlur={()=>setFocused(false)} onClick={()=>{setHovered(false);setFocused(false);window.dispatchEvent(new CustomEvent('musicos-quick-audio',{detail:{anchor:anchor.current,track:track??null}}));}} aria-describedby={tooltipId}>
 {!compact&&<span>{label}</span>}<Icon name={label==='Lossless'?'Lossless':label==='Lossy'?'Lossy':'that_one_i_in_a_circle_icon'}/></button>
 <span ref={tooltip} popover="manual" className="audio-specs" role="tooltip" id={tooltipId}><strong>{track?.title??'No music playing'}</strong>{loading?'Reading audio file…':error||!specs?(error||'Hover to read audio specifications'):<><span>{specs.format} · {specs.lossless?'Lossless':'Compressed audio'}</span><span>{specs.sample_rate?`${(specs.sample_rate/1000).toLocaleString()} kHz`:'Sample rate unavailable'}{specs.bit_depth?` · ${specs.bit_depth}-bit`:''}{specs.channels?` · ${specs.channels===1?'Mono':specs.channels===2?'Stereo':`${specs.channels} channels`}`:''}</span>{specs.bitrate&&<span>{specs.bitrate} kbps</span>}</>}{tracks.length>1&&<small>Specifications for the first song. {formats.length>1?'This album contains mixed formats; quality is shown per song.':''}</small>}</span></span>;
}
