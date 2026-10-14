import {useEffect,useState} from 'react';
import {call,native} from './bridge';
const requests=new Map<string,Promise<string>>();
export function usePortraitArtwork(source:string|null|undefined,revision=0){
 const [framed,setFramed]=useState<{source:string;path:string}|null>(null);
 useEffect(()=>{if(!native||!source||source.startsWith('https:')||source.startsWith('assets/')||source.startsWith('data:'))return;
  let alive=true;const key=`${source}:${revision}`;let request=requests.get(key);if(!request){request=call('portrait_artwork',{path:source});requests.set(key,request);request.catch(()=>requests.delete(key));}
  void request.then(path=>{if(alive)setFramed({source,path});}).catch(()=>{});return()=>{alive=false;};
 },[source,revision]);
 return framed&&framed.source===source?framed.path:source;
}
