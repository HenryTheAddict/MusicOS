import './playerTabs.css';
import{useLayoutEffect,useRef}from'react';
import{animateOwned,cancelOwned}from'./presentationMotion';
/** A moving fill, with invariant text metrics and stationary hit targets. */
export default function PlayerTabs<T extends string>({value,items,onChange,label,className}:{value:T;items:readonly {value:T;label:string}[];onChange:(v:T)=>void;label:string;className:string}){
 const root=useRef<HTMLDivElement>(null),fill=useRef<HTMLSpanElement>(null),previous=useRef<{left:number;width:number}|null>(null);
 useLayoutEffect(()=>{const host=root.current,pill=fill.current;if(!host||!pill)return;const place=(moving:boolean)=>{const active=host.querySelector<HTMLButtonElement>('[aria-pressed=true]');if(!active)return;const target={left:active.offsetLeft,width:active.offsetWidth},old=previous.current,current=getComputedStyle(pill);const from={transform:current.transform,width:current.width};if(!moving)cancelOwned(pill);Object.assign(pill.style,{left:'0px',transform:`translateX(${target.left}px)`,width:`${target.width}px`});if(moving&&old)animateOwned(pill,'player-tab-fill',[from,{transform:`translateX(${target.left}px)`,width:`${target.width}px`}],140);previous.current=target;};place(true);const resize=new ResizeObserver(()=>place(false));resize.observe(host);return()=>resize.disconnect();},[value]);
 useLayoutEffect(()=>()=>{if(fill.current)cancelOwned(fill.current);},[]);
 return <div ref={root} className={`${className} player-motion-tabs`} role="group" aria-label={label}><span ref={fill} className="player-tab-fill" aria-hidden="true"/>{items.map(item=><button key={item.value} aria-pressed={value===item.value} onClick={()=>{if(item.value!==value)onChange(item.value);}}>{item.label}</button>)}</div>;
}
