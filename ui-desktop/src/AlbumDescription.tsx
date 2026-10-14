import {useEffect,useId,useRef,useState} from 'react';
import {call} from './bridge';
export default function AlbumDescription({albumId}:{albumId:string}){
 const [info,setInfo]=useState<{description:string|null;source_url:string|null}|null>(null),[expanded,setExpanded]=useState(false);
 const paragraph=useRef<HTMLParagraphElement>(null),textId=useId();
 const [canExpand,setCanExpand]=useState(false);
 useEffect(()=>{let active=true;void call('album_info',{albumId}).then(value=>{if(active)setInfo(value);}).catch(()=>{});return()=>{active=false;};},[albumId]);
 useEffect(()=>{
  const element=paragraph.current;if(!element)return;
  const measure=()=>{if(!expanded)setCanExpand(element.scrollHeight>element.clientHeight+1);};
  measure();const observer=new ResizeObserver(measure);observer.observe(element);return()=>observer.disconnect();
 },[info,expanded]);
 if(!info?.description)return null;
 return <section className="album-description" aria-label="About this album"><h2>About this album</h2><p ref={paragraph} id={textId} data-expanded={expanded}>{info.description}</p><footer>{(canExpand||expanded)&&<button onClick={()=>setExpanded(!expanded)} aria-controls={textId} aria-expanded={expanded}>{expanded?'Read less':'Read more'}</button>}{info.source_url&&<button onClick={()=>void call('open_provider',{url:info.source_url!})}>Wikipedia · CC BY-SA ↗</button>}</footer></section>;
}
