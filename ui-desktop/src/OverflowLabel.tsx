import {useLayoutEffect,useRef,useState,type CSSProperties} from 'react';
/** A quiet trailing fade, with one deliberate hover/focus reveal and no idle ticker. */
export default function OverflowLabel({text}:{text:string}){
 const host=useRef<HTMLSpanElement>(null),content=useRef<HTMLSpanElement>(null);
 const [overflow,setOverflow]=useState(0);
 useLayoutEffect(()=>{
  const measure=()=>{if(host.current&&content.current)setOverflow(Math.max(0,content.current.scrollWidth-host.current.clientWidth));};
  const observer=new ResizeObserver(measure);if(host.current)observer.observe(host.current);if(content.current)observer.observe(content.current);measure();
  return()=>observer.disconnect();
 },[text]);
 return <span ref={host} className="overflow-label" tabIndex={overflow>1&&!host.current?.closest('button')?0:undefined} data-overflow={overflow>1} style={{'--label-offset':`${-overflow}px`,'--label-duration':`${Math.max(900,Math.min(12000,overflow/35*1000))}ms`} as CSSProperties}><span ref={content}>{text}</span></span>;
}
