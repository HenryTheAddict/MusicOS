import{listen}from'@tauri-apps/api/event';
import{useEffect,useRef,useState,type ReactNode}from'react';
import{native}from'./bridge';
import{appleCall}from'./AppleEnrichment';
import{useViewActive}from'./ViewScope';
const owners=new Map<symbol,{priority:number;visible:boolean;set:(owned:boolean)=>void}>();
function elect(){const winner=[...owners].filter(([,v])=>v.visible).sort((a,b)=>b[1].priority-a[1].priority)[0]?.[0];for(const[id,o]of owners)o.set(id===winner);}
const requests=new Map<number,Promise<string|null>>();
let connected=false;let connectionCheck:Promise<unknown>|undefined;
window.addEventListener('musicos-apple-status',e=>{connected=!!(e as CustomEvent).detail?.signed_in;if(!connected)requests.clear();});
export default function MotionArtwork({trackId,priority,children,enabled=true}:{trackId?:number;priority:number;children:ReactNode;enabled?:boolean}){
 const active=useViewActive(),node=useRef<HTMLSpanElement>(null),video=useRef<HTMLVideoElement>(null),[owned,setOwned]=useState(false),[url,setUrl]=useState<string|null>(null),[failed,setFailed]=useState(false),[epoch,setEpoch]=useState(0),[viewable,setViewable]=useState(false);
 useEffect(()=>{if(native&&active&&!connectionCheck)connectionCheck=appleCall('status').catch(()=>{});},[active]);
 useEffect(()=>{const host=node.current;if(!host)return;const observer=new IntersectionObserver(entries=>setViewable(!!entries[0]?.isIntersecting));observer.observe(host);return()=>observer.disconnect();},[]);
 useEffect(()=>{const update=()=>setEpoch(n=>n+1);window.addEventListener('musicos-apple-status',update);return()=>window.removeEventListener('musicos-apple-status',update);},[]);
 useEffect(()=>{if(!enabled||!active||!native||!trackId||!connected||!viewable){setUrl(null);return;}let alive=true;let job=requests.get(trackId);if(!job){job=appleCall('artwork',trackId).then(r=>r.url??null).catch(()=>null);requests.set(trackId,job);if(requests.size>100)requests.delete(requests.keys().next().value!);}setFailed(false);setUrl(null);void job.then(value=>{if(alive)setUrl(value);});return()=>{alive=false;};},[trackId,enabled,active,epoch,viewable]);
 useEffect(()=>{const host=node.current;if(!host||!enabled||!active||!url||failed)return;const id=Symbol(),reduced=matchMedia('(prefers-reduced-motion: reduce)');let intersecting=false,power=true,alive=true,off:(()=>void)|undefined;const update=()=>{owners.set(id,{priority,visible:intersecting&&!document.hidden&&!reduced.matches&&!power,set:setOwned});elect();};const observer=new IntersectionObserver(entries=>{intersecting=entries[0]?.isIntersecting??false;update();});observer.observe(host);void listen<{lowPower:boolean}>('musicos-motion-policy',({payload})=>{power=payload.lowPower;update();}).then(dispose=>{if(alive)off=dispose;else dispose();}).catch(()=>{});document.addEventListener('visibilitychange',update);reduced.addEventListener('change',update);void appleCall('motion_policy').then(r=>{if(alive){power=!!r.lowPower;update();}});return()=>{alive=false;off?.();observer.disconnect();document.removeEventListener('visibilitychange',update);reduced.removeEventListener('change',update);owners.delete(id);setOwned(false);elect();};},[url,enabled,active,priority,failed]);
 useEffect(()=>{const v=video.current;if(owned&&v){void v.play().catch(()=>setFailed(true));return()=>{v.pause();v.removeAttribute('src');v.load();};}},[owned,url]);
 return <span ref={node} className="motion-artwork">{children}{owned&&url&&!failed&&<video ref={video} src={url} muted loop playsInline preload="none" disablePictureInPicture aria-hidden="true" onLoadedData={e=>{const cover=node.current?.querySelector('.cover');if(cover){const style=getComputedStyle(cover);e.currentTarget.style.borderRadius=style.borderRadius;e.currentTarget.style.clipPath=style.clipPath;}}} onError={()=>setFailed(true)}/>}</span>;
}
