import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {Icon,type MenuItem} from './components';

// A themed menu needs room below the player. Expand the native window
// without resizing the player itself, and collapse it before running an action.
export default function MiniActions({items,height,onResize,contextOnly=false}:{contextOnly?:boolean;items:MenuItem[];height:number;onResize:(open:boolean)=>Promise<void>}){
 const opener=useRef<HTMLButtonElement>(null),panel=useRef<HTMLDivElement>(null);
 const [open,setOpen]=useState(false),[closing,setClosing]=useState(false),[error,setError]=useState(''),[layoutHeight,setLayoutHeight]=useState(height);
 const exit=useRef<Animation|null>(null);
 const generation=useRef(0),wanted=useRef(false),alive=useRef(true),chain=useRef(Promise.resolve());
 const resize=(expanded:boolean)=>{const next=chain.current.catch(()=>{}).then(()=>onResize(expanded));chain.current=next;return next;};
 async function close(action?:()=>void){
  if(!wanted.current)return;const request=++generation.current;wanted.current=false;setClosing(true);
  try{
   if(panel.current&&!matchMedia('(prefers-reduced-motion: reduce)').matches&&panel.current.animate){exit.current=panel.current.animate([{opacity:getComputedStyle(panel.current).opacity,transform:getComputedStyle(panel.current).transform},{opacity:0,transform:'scale(.98)'}],{duration:90,easing:'ease-out',fill:'forwards'});await exit.current.finished.catch(()=>{});}
   if(wanted.current||generation.current!==request)return;
   await resize(false);
   if(wanted.current||generation.current!==request)return;
   setOpen(false);setClosing(false);exit.current?.cancel();exit.current=null;
   opener.current?.closest<HTMLElement>('.mini-player')?.style.removeProperty('--mini-player-height');
   if(alive.current){if(contextOnly)opener.current?.closest<HTMLElement>('.lyrics-pane')?.focus();else opener.current?.focus();}action?.();
  }catch{if(alive.current){setClosing(false);setError('Could not close actions. Try again.');}}
 }
 async function show(){if(wanted.current)return;wanted.current=true;const style=panel.current?getComputedStyle(panel.current):null;const opacity=style?.opacity??'0',transform=style?.transform??'scale(.96)';exit.current?.cancel();exit.current=open&&panel.current&&!matchMedia('(prefers-reduced-motion: reduce)').matches?panel.current.animate([{opacity,transform},{opacity:1,transform:'scale(1)'}],{duration:140,easing:'ease-out'}):null;setClosing(false);const request=++generation.current;setError('');const root=opener.current?.closest<HTMLElement>('.mini-player');const actual=root?.getBoundingClientRect().height??height;setLayoutHeight(actual);root?.style.setProperty('--mini-player-height',`${actual}px`);try{await resize(true);if(alive.current&&wanted.current&&generation.current===request)setOpen(true);}catch{wanted.current=false;root?.style.removeProperty('--mini-player-height');if(alive.current)setError('Could not open actions. Try again.');}}
 const showRef=useRef(show);showRef.current=show;
 useEffect(()=>{alive.current=true;const button=opener.current;const request=()=>void showRef.current();button?.addEventListener('musicos-open-menu',request);return()=>{alive.current=false;wanted.current=false;exit.current?.cancel();button?.removeEventListener('musicos-open-menu',request);};},[]);
 useLayoutEffect(()=>{if(!open||!panel.current)return;const node=panel.current;const place=()=>{const below=innerHeight-layoutHeight-12;if(below<80){const anchor=opener.current?.getBoundingClientRect();const bottom=Math.max(48,(anchor?.top??layoutHeight-80)-4);node.style.top=`${Math.max(8,bottom-232)}px`;node.style.maxHeight=`${Math.min(232,bottom-8)}px`;node.dataset.side='top';}else{node.style.top=`${layoutHeight+4}px`;node.style.maxHeight=`${Math.min(232,below)}px`;node.dataset.side='bottom';}node.style.transformOrigin=`${Math.max(0,Math.min(innerWidth-16,(opener.current?.getBoundingClientRect().left??innerWidth)-8))}px ${node.dataset.side==='top'?'100%':'0'}`;};place();window.addEventListener('resize',place);node.showPopover();node.querySelector<HTMLButtonElement>('[role=menuitem]:not(:disabled)')?.focus();const outside=(event:PointerEvent)=>{if(!node.contains(event.target as Node)&&!opener.current?.contains(event.target as Node))void close();};window.addEventListener('pointerdown',outside);return()=>{node.hidePopover();window.removeEventListener('resize',place);window.removeEventListener('pointerdown',outside);};},[open,layoutHeight]);
 return <><button ref={opener} className={`menu-trigger ${contextOnly?'context-only':''}`} tabIndex={contextOnly?-1:undefined} aria-hidden={contextOnly||undefined} aria-label="Mini Player actions" aria-haspopup="menu" aria-expanded={open} onClick={()=>void(wanted.current?close():show())}><Icon name={open&&!closing?'menuDotsClose':'3dots(incircle)'}/></button>
 <div ref={panel} popover="manual" hidden={!open} inert={closing} data-closing={closing} role="menu" aria-label="Mini Player actions" className="dropdown mini-actions-popover" style={{top:layoutHeight+4,maxHeight:`min(232px, calc(100dvh - ${layoutHeight+12}px))`}} onKeyDown={event=>{const buttons=[...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role=menuitem]:not(:disabled)')];const index=buttons.indexOf(document.activeElement as HTMLButtonElement);if(event.key==='Escape'||event.key==='Tab'){event.preventDefault();event.stopPropagation();void close();}else if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();buttons[event.key==='Home'?0:event.key==='End'?buttons.length-1:(index+(event.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length]?.focus();}}}>
 {items.map((item,index)=><button key={`${item.label}-${index}`} role="menuitem" className="dropdown-item" disabled={item.disabled} onClick={()=>void close(item.onSelect)}>{item.icon&&<Icon name={item.icon}/>}<span>{item.label}</span></button>)}
 </div>{error&&<span className="mini-player-error" role="alert">{error}</span>}</>;
}
