import {useEffect,useState} from 'react';
export default function usePanelPresence(open:boolean){
 const [retained,setRetained]=useState(open);
 useEffect(()=>{
  if(open){setRetained(true);return;}
  const timer=setTimeout(()=>setRetained(false),window.matchMedia('(prefers-reduced-motion: reduce)').matches?0:180);
  return()=>clearTimeout(timer);
 },[open]);
 return open||retained;
}
