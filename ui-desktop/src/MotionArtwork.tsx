import {listen} from '@tauri-apps/api/event';
import {useEffect,useRef,useState,type ReactNode} from 'react';
import {artwork,native} from './bridge';
import {appleCall,type AppleResponse} from './AppleEnrichment';
import {artworkRendition} from './useArtworkRendition';
import {useViewActive} from './ViewScope';
const owners=new Map<symbol,{priority:number;visible:boolean;set:(owned:boolean)=>void}>();
function elect(){const winner=[...owners].filter(([,v])=>v.visible).sort((a,b)=>b[1].priority-a[1].priority)[0]?.[0];for(const[id,o]of owners)o.set(id===winner);}
type Assets={url:string|null;still:string|null};
const requests=new Map<string,{expires:number;promise:Promise<Assets>}>();
let connected=false,connectionCheck:Promise<AppleResponse>|undefined;
window.addEventListener('musicos-apple-status',e=>{connected=!!(e as CustomEvent).detail?.signed_in;requests.clear();if(!connected)connectionCheck=undefined;});
function assets(trackId:number,expanded:boolean){
 const key=`${trackId}:${expanded}`,cached=requests.get(key);if(cached&&cached.expires>Date.now())return cached.promise;
 const entry={expires:Date.now()+60_000,promise:Promise.resolve<Assets>({url:null,still:null})};
 entry.promise=appleCall(expanded?'expanded_artwork':'artwork',trackId).then(async r=>{
  const still=r.staticPath?await artworkRendition(r.staticPath,2048).catch(()=>r.staticPath!):null;
  if(r.url||still)entry.expires=Date.now()+3600_000;
  return{url:r.url??null,still};
 }).catch(()=>({url:null,still:null}));
 if(requests.size>=100)requests.delete(requests.keys().next().value!);requests.set(key,entry);return entry.promise;
}
export default function MotionArtwork({trackId,priority,children,enabled=true,animated=true}:{trackId?:number;priority:number;children:ReactNode;enabled?:boolean;animated?:boolean}){
 const active=useViewActive(),node=useRef<HTMLSpanElement>(null),video=useRef<HTMLVideoElement>(null);
 const [owned,setOwned]=useState(false),[resolved,setResolved]=useState<{id:number;assets:Assets}|null>(null),[failed,setFailed]=useState(false),[videoReady,setVideoReady]=useState(false),[stillReady,setStillReady]=useState(false),[epoch,setEpoch]=useState(0),[viewable,setViewable]=useState(false);
 const current=resolved&&resolved.id===trackId?resolved.assets:null,url=current?.url??null,still=current?.still??null;
 useEffect(()=>{if(native&&active&&!connectionCheck){connectionCheck=appleCall('status');void connectionCheck.catch(()=>{connectionCheck=undefined;});}},[active]);
 useEffect(()=>{const host=node.current;if(!host)return;const observer=new IntersectionObserver(entries=>setViewable(!!entries[0]?.isIntersecting));observer.observe(host);return()=>observer.disconnect();},[]);
 useEffect(()=>{const update=()=>setEpoch(n=>n+1);window.addEventListener('musicos-apple-status',update);return()=>window.removeEventListener('musicos-apple-status',update);},[]);
 useEffect(()=>{
  if(!enabled||!active||!native||!trackId||!connected||!viewable)return;
  let alive=true;setFailed(false);setVideoReady(false);setStillReady(false);
  void assets(trackId,priority>=3).then(value=>{if(alive)setResolved({id:trackId,assets:value});});
  return()=>{alive=false;};
 },[trackId,enabled,active,epoch,viewable,priority]);
 useEffect(()=>{
  const host=node.current;if(!host||!animated||!enabled||!active||!url||failed)return;
  const id=Symbol(),reduced=matchMedia('(prefers-reduced-motion: reduce)');let intersecting=false,power=true,alive=true,off:(()=>void)|undefined;
  const update=()=>{owners.set(id,{priority,visible:intersecting&&!document.hidden&&!reduced.matches&&!power,set:setOwned});elect();};
  const observer=new IntersectionObserver(entries=>{intersecting=entries[0]?.isIntersecting??false;update();});observer.observe(host);
  void listen<{lowPower:boolean}>('musicos-motion-policy',({payload})=>{power=payload.lowPower;update();}).then(dispose=>{if(alive)off=dispose;else dispose();}).catch(()=>{});
  document.addEventListener('visibilitychange',update);reduced.addEventListener('change',update);
  void appleCall('motion_policy').then(r=>{if(alive){power=!!r.lowPower;update();}}).catch(()=>{});
  return()=>{alive=false;off?.();observer.disconnect();document.removeEventListener('visibilitychange',update);reduced.removeEventListener('change',update);owners.delete(id);setOwned(false);elect();};
 },[url,animated,enabled,active,priority,failed]);
 useEffect(()=>{const v=video.current;if(owned&&v){void v.play().catch(()=>setFailed(true));return()=>{v.pause();v.removeAttribute('src');v.load();};}},[owned,url]);
 function inheritGeometry(element:HTMLElement){const cover=node.current?.querySelector('.cover');if(cover){const style=getComputedStyle(cover);element.style.borderRadius=style.borderRadius;element.style.clipPath=style.clipPath;}}
 return <span ref={node} className="motion-artwork">{children}
  {enabled&&still&&<img key={still} className="expanded-static-artwork" src={artwork(still)} style={{opacity:stillReady?1:0}} draggable={false} aria-hidden="true" alt="" onLoad={e=>{inheritGeometry(e.currentTarget);setStillReady(true);}}/>}
  {owned&&url&&!failed&&<video key={url} ref={video} src={url} style={{opacity:videoReady?1:0}} muted loop playsInline preload="none" disablePictureInPicture aria-hidden="true" onLoadedData={e=>{inheritGeometry(e.currentTarget);setVideoReady(true);}} onError={()=>setFailed(true)}/>}
 </span>;
}
