import {useEffect,useRef,useState,type RefObject} from 'react';
import {call,native} from './bridge';
import {useViewActive} from './ViewScope';
const requests=new Map<string,Promise<string>>();
const recent=new Map<string,string>();
const renditionKey=(source:string,revision:number)=>JSON.stringify([source,revision]);

function rememberRendition(source:string,revision:number,path:string){
 const key=renditionKey(source,revision);
 recent.delete(key);
 if(recent.size>=256)recent.delete(recent.keys().next().value!);
 recent.set(key,path);
 return path;
}
export function artworkRendition(source:string,pixels:number,revision=0){
 if(!native||source.startsWith('data:')||source.startsWith('assets/')||source.startsWith('/assets/'))return Promise.resolve(source);
 const key=`${source}:${revision}:${pixels}`;let request=requests.get(key);
 if(!request){
  request=call('artwork_rendition',{path:source,pixels}).then(path=>rememberRendition(source,revision,path));
  if(requests.size>=2048)requests.delete(requests.keys().next().value!);
  requests.set(key,request);request.catch(()=>requests.delete(key));
 }
 return request;
}
export function useArtworkRendition(source:string|null|undefined,revision:number,image:RefObject<HTMLImageElement|null>,failed:()=>void){
 const cacheable=!!source&&!source.startsWith('data:')&&!source.startsWith('assets/')&&!source.startsWith('/assets/');
 const active=useViewActive(),failure=useRef(failed);failure.current=failed;
 const [ready,setReady]=useState<{source:string;revision:number;path:string}|null>(null);
 useEffect(()=>{
  if(!native||!cacheable||!active||!source||!image.current)return;
  let alive=true,near=typeof IntersectionObserver==='undefined',last=0,serial=0;
  const element=image.current;
  const update=()=>{
   if(!near)return;
   const pixels=Math.max(element.clientWidth,element.clientHeight)*Math.min(window.devicePixelRatio||1,2);
   const size=!pixels?640:pixels<=128?128:pixels<=640?640:pixels<=1200?1200:2048;
   if(size===last)return;last=size;const current=++serial;
   const request=artworkRendition(source,size,revision);
   void request.then(path=>{if(alive&&current===serial)setReady({source,revision,path});}).catch(()=>{if(alive&&current===serial)failure.current();});
  };
  const resize=typeof ResizeObserver==='undefined'?null:new ResizeObserver(update);resize?.observe(element);
  const visibility=typeof IntersectionObserver==='undefined'?null:new IntersectionObserver(entries=>{near=entries.some(entry=>entry.isIntersecting);if(near)update();},{rootMargin:'200px'});
  visibility?.observe(element);update();
  return()=>{alive=false;resize?.disconnect();visibility?.disconnect();};
 },[source,revision,image,active,cacheable]);
 return !native||!cacheable?source:ready&&ready.source===source&&ready.revision===revision?ready.path:recent.get(renditionKey(source!,revision))??null;
}
