import {useLayoutEffect,useRef,useState} from 'react';
export type ShellLayout='wide'|'compact';
export function shellLayout(width:number,fontSize:number):ShellLayout{return width<Math.min(720,40*fontSize)?'compact':'wide';}
/** One scale-aware measurement owns all shell presentation decisions. */
export function useShellLayout(scale:number,enabled=true){
 const root=useRef<HTMLDivElement>(null);
 const [layout,setLayout]=useState<ShellLayout>(()=>shellLayout(innerWidth,16*scale));
 useLayoutEffect(()=>{
  const node=root.current;if(!node||!enabled)return;
  let frame=0;
  const measure=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>setLayout(shellLayout(node.clientWidth,16*scale)));};
  const observer=new ResizeObserver(measure);observer.observe(node);measure();
  return()=>{observer.disconnect();cancelAnimationFrame(frame);};
 },[scale,enabled]);
 return {root,layout,compact:layout==='compact'};
}
