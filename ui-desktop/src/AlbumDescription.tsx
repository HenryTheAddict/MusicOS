import {useEffect,useId,useRef,useState} from 'react';
import SecondaryAction from './SecondaryAction';
import {call} from './bridge';
export default function AlbumDescription({albumId}:{albumId:string}){
 const [info,setInfo]=useState<{description:string|null;source_url:string|null}|null>(null),[expanded,setExpanded]=useState(false);
 const paragraph=useRef<HTMLParagraphElement>(null),dialog=useRef<HTMLDialogElement>(null),opener=useRef<HTMLButtonElement>(null),textId=useId();
 const [canExpand,setCanExpand]=useState(false);
 useEffect(()=>{let active=true;void call('album_info',{albumId}).then(value=>{if(active)setInfo(value);}).catch(()=>{});return()=>{active=false;};},[albumId]);
 useEffect(()=>{const element=paragraph.current;if(!element)return;const measure=()=>setCanExpand(element.scrollHeight>element.clientHeight+1);measure();const observer=new ResizeObserver(measure);observer.observe(element);return()=>observer.disconnect();},[info]);
 useEffect(()=>{if(!expanded)return;dialog.current?.showModal();return()=>{dialog.current?.close();opener.current?.focus();};},[expanded]);
 if(!info?.description)return null;
 return <section className="album-description" aria-label="About this album"><h2>About this album</h2><p ref={paragraph} id={textId} data-expanded="false">{info.description}</p><footer>{canExpand&&<SecondaryAction ref={opener} icon="that_one_i_in_a_circle_icon" onClick={()=>setExpanded(true)} aria-haspopup="dialog" aria-expanded={expanded}>Read more</SecondaryAction>}{info.source_url&&<button onClick={()=>void call('open_provider',{url:info.source_url!})}>Wikipedia · CC BY-SA ↗</button>}</footer>{expanded&&<dialog ref={dialog} className="album-about-dialog" aria-label="About this album" onCancel={e=>{e.preventDefault();setExpanded(false);}}><header><h2>About this album</h2><button aria-label="Close album description" onClick={()=>setExpanded(false)}>×</button></header><p>{info.description}</p>{info.source_url&&<button onClick={()=>void call('open_provider',{url:info.source_url!})}>Wikipedia · CC BY-SA ↗</button>}</dialog>}</section>;
}
