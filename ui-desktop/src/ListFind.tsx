import {useId,useLayoutEffect,useRef,useState,useEffect} from 'react';
import {Icon} from './components';
import usePanelPresence from './usePanelPresence';
const examples=[['two words','Match both words'],['"exact phrase"','Keep words together'],['artist:M83','Find an artist'],['album:Discovery','Find an album'],['year:2010-2015','Choose a year range'],['is:favorite','Only favorites'],['-live','Exclude a word']];
export default function ListFind({open,focusRequest,value,context,count,total,unit,onChange,onClose}:{open:boolean;focusRequest:number;value:string;context:string;count:number;total:number;unit:string;onChange:(value:string)=>void;onClose:()=>void}){
 const present=usePanelPresence(open),input=useRef<HTMLInputElement>(null),help=useId(),tips=useRef<HTMLDivElement>(null),tipButton=useRef<HTMLButtonElement>(null);
 const [showTips,setShowTips]=useState(false);
 useLayoutEffect(()=>{if(open){input.current?.focus({preventScroll:true});input.current?.select();}else setShowTips(false);},[open,focusRequest]);
 useEffect(()=>{if(!showTips)return;const outside=(event:PointerEvent)=>{if(!tips.current?.contains(event.target as Node))setShowTips(false);};document.addEventListener('pointerdown',outside,true);return()=>document.removeEventListener('pointerdown',outside,true);},[showTips]);
 return present?<div id="list-find" className="list-find" role="search" aria-label={`Find in ${context}`} data-open={open} inert={!open} aria-hidden={!open} onKeyDown={event=>{if(event.key==='Escape'){event.preventDefault();event.stopPropagation();if(showTips){setShowTips(false);tipButton.current?.focus();}else onClose();}}}>
  <label className="list-find-field"><Icon name="search"/><input ref={input} type="search" aria-label={`Find in ${context}`} placeholder={`Find in ${context}`} value={value} onChange={event=>onChange(event.target.value)} autoComplete="off" spellCheck={false}/></label>
  <span className="list-find-count" role="status" aria-live="polite">{count} of {total} {unit}{total===1?'':'s'}</span>
  {value&&<button className="list-find-clear" onClick={()=>{onChange('');input.current?.focus();}}>Clear</button>}
  <button className="list-find-close" aria-label="Close Find" title="Close Find (Escape)" onClick={onClose}><Icon name="tinyX"/></button>
  <div className="list-find-tips" ref={tips}><button ref={tipButton} aria-expanded={showTips} aria-controls={help} onClick={()=>setShowTips(value=>!value)}>Tips</button>{showTips&&<section id={help} className="find-help" aria-label="Find tips"><h2>Find in this list</h2><dl>{examples.map(([example,meaning])=><div key={example}><dt><code>{example}</code></dt><dd>{meaning}</dd></div>)}</dl><p>Small spelling mistakes are okay.</p></section>}</div>
 </div>:null;
}
