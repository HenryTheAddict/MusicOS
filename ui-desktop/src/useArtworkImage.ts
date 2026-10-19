import {useLayoutEffect,useRef,useState} from 'react';
export type ArtworkImage={url:string;path?:string;original?:string;version:number};
// Publish the URL and depth identity together, after decode. Routine analysis
// must not replace an already visible cover with a loading frame.
export function useArtworkImage(wanted:ArtworkImage,identity:string,keepCurrent:boolean,failed:()=>void,pending=false){
 const [shown,setShown]=useState({identity,image:wanted});
 const failure=useRef(failed);failure.current=failed;
 useLayoutEffect(()=>{
  if(shown.identity!==identity||!keepCurrent){setShown({identity,image:wanted});return;}
  if(pending)return;
  if(shown.image.url===wanted.url&&shown.image.path===wanted.path&&shown.image.version===wanted.version)return;
  let alive=true;const next=new Image();
  const decoded=typeof next.decode==='function'?(()=>{next.src=wanted.url;return next.decode();})():new Promise<void>((resolve,reject)=>{next.onload=()=>resolve();next.onerror=()=>reject();next.src=wanted.url;if(next.complete&&next.naturalWidth)resolve();});
  void decoded.then(()=>{if(alive)setShown({identity,image:wanted});}).catch(()=>{if(alive)failure.current();});
  return()=>{alive=false;};
 },[wanted.url,wanted.path,wanted.original,wanted.version,identity,keepCurrent,pending]);
 return shown.identity===identity&&keepCurrent?shown.image:wanted;
}
