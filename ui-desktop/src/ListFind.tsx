import {useId,useLayoutEffect,useRef} from 'react';
import {Icon} from './components';
import usePanelPresence from './usePanelPresence';
export default function ListFind({open,focusRequest,value,context,count,total,unit,onChange,onClose}:{open:boolean;focusRequest:number;value:string;context:string;count:number;total:number;unit:string;onChange:(value:string)=>void;onClose:()=>void}){
 const present=usePanelPresence(open),input=useRef<HTMLInputElement>(null),help=useId();
 useLayoutEffect(()=>{if(open){input.current?.focus({preventScroll:true});input.current?.select();}},[open,focusRequest]);
 return present?<div id="list-find" className="list-find" role="search" aria-label={`Find in ${context}`} data-open={open} inert={!open} aria-hidden={!open} onKeyDown={event=>{if(event.key==='Escape'){event.preventDefault();event.stopPropagation();onClose();}}}>
  <label className="list-find-field"><Icon name="search"/><input ref={input} type="search" aria-label={`Find in ${context}`} aria-describedby={help} placeholder={`Find in ${context}`} value={value} onChange={event=>onChange(event.target.value)} autoComplete="off" spellCheck={false}/></label>
  <span className="list-find-count" role="status" aria-live="polite">{count} of {total} {unit}{total===1?'':'s'}</span>
  {value&&<button className="list-find-clear" onClick={()=>{onChange('');input.current?.focus();}}>Clear</button>}
  <button className="list-find-close" aria-label="Close Find" title="Close Find (Escape)" onClick={onClose}><Icon name="tinyX"/></button>
  <details className="list-find-tips"><summary>Tips</summary><p id={help}>Use several words, “quoted phrases”, artist:M83, album:Discovery, year:2010-2015, or is:favorite. Add a minus to exclude a term.</p></details>
 </div>:null;
}
