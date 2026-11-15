import {useLayoutEffect,useRef,useState} from 'react';
export type ShellLayout='wide'|'compact';
export function shellLayout(width:number,fontSize:number):ShellLayout{return width<Math.min(720,40*fontSize)?'compact':'wide';}

function presentation(width:number,scale:number){
 const unit=16*scale;
 return {layout:shellLayout(width,unit),dockLibrary:width>=60*unit,dockSection:width>=47*unit};
}

/** React only updates when a layout decision changes, even during live resize. */
export function useShellLayout(scale:number,enabled=true){
 const root=useRef<HTMLDivElement>(null);
 const [size,setSize]=useState(()=>presentation(innerWidth,scale));
 useLayoutEffect(()=>{
  const node=root.current;
  if(!node||!enabled)return;
  let frame=0;
  const measure=()=>{
   cancelAnimationFrame(frame);
   frame=requestAnimationFrame(()=>{
    const next=presentation(node.clientWidth,scale);
    setSize(old=>next.layout===old.layout&&next.dockLibrary===old.dockLibrary&&next.dockSection===old.dockSection?old:next);
   });
  };
  const observer=new ResizeObserver(measure);
  observer.observe(node);
  measure();
  return()=>{observer.disconnect();cancelAnimationFrame(frame);};
 },[scale,enabled]);
 return {root,...size,compact:size.layout==='compact'};
}
