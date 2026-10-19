import {useEffect,useState} from 'react';
import {call,native} from './bridge';
type Portrait={path:string;display:string};
const requests=new Map<string,Promise<Portrait>>();
export function usePortraitArtwork(source:string|null|undefined,revision=0){
 const direct=!native||!!source&&(source.startsWith('https:')||source.startsWith('assets/')||source.startsWith('/assets/')||source.startsWith('data:'));
 const [framed,setFramed]=useState<{source:string;photo:Portrait}|null>(null);
 useEffect(()=>{
  if(direct||!source)return;
  let alive=true;const key=`${source}:${revision}`;let request=requests.get(key);
  if(!request){
   request=call('portrait_artwork',{path:source}).catch(()=>source).then(async path=>({path,display:await call('artwork_rendition',{path,pixels:1200})}));
   if(requests.size>=512)requests.delete(requests.keys().next().value!);requests.set(key,request);request.catch(()=>requests.delete(key));
  }
  void request.then(photo=>{if(alive)setFramed({source,photo});}).catch(()=>{});return()=>{alive=false;};
 },[source,revision,direct]);
 return direct&&source?{path:source,display:source}:framed&&framed.source===source?framed.photo:null;
}
