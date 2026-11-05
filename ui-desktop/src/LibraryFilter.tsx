import {useEffect,useLayoutEffect,useRef} from 'react';
import {useMenuPresentation} from './menuPresentation';
import * as Dropdown from '@radix-ui/react-dropdown-menu';
import {Icon} from './components';
import ThemedSwitch from './ThemedSwitch';
import type {Library,LibraryFilters} from './types';
export const defaultFilters:LibraryFilters={sort:'recent',genre:'',quality:'all',favorites:false,availability:'all',separate_singles:false};

type Choice = {value:string;label:string};
function FilterChoice({label,value,choices,onChange}:{label:string;value:string;choices:Choice[];onChange:(value:string)=>void}) {
 return <div className="filter-field"><span>{label}</span><Dropdown.Root modal={false}>
  <Dropdown.Trigger className="filter-choice" aria-label={label}>
   <span>{choices.find(choice=>choice.value===value)?.label ?? value}</span>
   <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m4 6 4 4 4-4" strokeLinecap="round" strokeLinejoin="round"/></svg>
  </Dropdown.Trigger>
  {/* Keep the option menu inside the native popover's top layer. */}
  <Dropdown.Content className="filter-options" sideOffset={5} align="start" collisionPadding={8} aria-label={`${label} options`} onEscapeKeyDown={e=>e.stopPropagation()}>
   <Dropdown.RadioGroup value={value} onValueChange={onChange}>
    {choices.map(choice=><Dropdown.RadioItem className="filter-option" key={choice.value} value={choice.value}>
     <span>{choice.label}</span><Dropdown.ItemIndicator><svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden="true"><path d="m3 8 3 3 7-7" strokeLinecap="round" strokeLinejoin="round"/></svg></Dropdown.ItemIndicator>
    </Dropdown.RadioItem>)}
   </Dropdown.RadioGroup>
  </Dropdown.Content>
 </Dropdown.Root></div>;
}

export default function LibraryFilter({library,value,onChange}:{library:Library;value:LibraryFilters;onChange:(value:LibraryFilters)=>void}){
 const root=useRef<HTMLDivElement>(null),trigger=useRef<HTMLButtonElement>(null);
 const menu=useMenuPresentation(trigger),{open,present,setOpen}=menu;
 useLayoutEffect(()=>{
  if(!present||!menu.content.current||!trigger.current)return;
  const node=menu.content.current;node.showPopover();
  const box=trigger.current.getBoundingClientRect();
  node.style.left=`${Math.max(8,Math.min(box.left,innerWidth-node.offsetWidth-8))}px`;
  const above=box.bottom+4+node.offsetHeight>innerHeight-8&&box.top>node.offsetHeight+8;
  node.dataset.side=above?'top':'bottom';node.style.top=`${above?box.top-node.offsetHeight-4:Math.min(box.bottom+4,Math.max(8,innerHeight-node.offsetHeight-8))}px`;
  const dismiss=()=>setOpen(false);
  window.addEventListener('resize',dismiss);
  return()=>{window.removeEventListener('resize',dismiss);if(node.matches(':popover-open'))node.hidePopover();};
 },[present]);
 useEffect(()=>{
  if(!open)return;
  const close=(e:PointerEvent)=>{if(!root.current?.contains(e.target as Node))setOpen(false);};
  const escape=(e:KeyboardEvent)=>{if(e.key==='Escape'&&!e.defaultPrevented){e.preventDefault();setOpen(false);trigger.current?.focus();}};
  window.addEventListener('pointerdown',close);window.addEventListener('keydown',escape);
  return()=>{window.removeEventListener('pointerdown',close);window.removeEventListener('keydown',escape);};
 },[open]);
 function set<K extends keyof LibraryFilters>(key:K,next:LibraryFilters[K]){onChange({...value,[key]:next});}
 return <div className="library-filter" ref={root}>
  <button className="library-filter-trigger" ref={trigger} aria-expanded={open} aria-controls="library-filter-panel" onClick={()=>setOpen(!open)}><Icon name="filter"/><span>Filter</span></button>
  {present&&<div ref={menu.ref} popover="manual" inert={!open} data-state={open?'open':'closed'} onToggle={e=>{if(e.newState==='closed')setOpen(false);}} id="library-filter-panel" className="library-filter-panel menu-panel" role="region" aria-label="Library filters"><div className="menu-panel-surface"/><div className="menu-panel-items">
   <FilterChoice label="Sort" value={value.sort} onChange={v=>set('sort',v as LibraryFilters['sort'])} choices={[{value:'recent',label:'Recently Added'},{value:'title',label:'Album Name'}]}/>
   <FilterChoice label="Genre" value={value.genre} onChange={v=>set('genre',v)} choices={[{value:'',label:'All genres'},...library.genres.map(g=>({value:g.name,label:g.name}))]}/>
   <FilterChoice label="Quality" value={value.quality} onChange={v=>set('quality',v as LibraryFilters['quality'])} choices={['all','lossless','lossy','mixed'].map(v=>({value:v,label:v==='all'?'All qualities':v[0].toUpperCase()+v.slice(1)}))}/>
   <FilterChoice label="Availability" value={value.availability} onChange={v=>set('availability',v as LibraryFilters['availability'])} choices={[{value:'all',label:'All songs'},{value:'available',label:'Available'},{value:'unavailable',label:'Includes unavailable songs'}]}/>
   <div className="filter-toggles"><ThemedSwitch label="Favorites only" checked={value.favorites} onChange={v=>set('favorites',v)}/><ThemedSwitch label="Separate Singles & EPs" checked={value.separate_singles} onChange={v=>set('separate_singles',v)}/></div>
   <div className="filter-footer"><button className="filter-reset" onClick={()=>onChange(defaultFilters)}>Reset filters</button></div>
  </div></div>}
 </div>;
}
export function filterAlbums(library:Library,filters:LibraryFilters){const tracks=new Map(library.tracks.map(t=>[t.id,t]));return library.albums.filter(a=>{const songs=a.track_ids.map(id=>tracks.get(id)).filter(t=>!!t);const quality=songs.length&&songs.every(t=>!!t.audio_quality)?songs.every(t=>t.audio_quality!.lossless)?'lossless':songs.every(t=>!t.audio_quality!.lossless)?'lossy':'mixed':'unknown';return (!filters.genre||songs.some(t=>t.genre===filters.genre))&&(filters.quality==='all'||filters.quality===quality)&&(!filters.favorites||songs.some(t=>t.favorited))&&(filters.availability==='all'||(filters.availability==='available'?songs.some(t=>t.available):songs.some(t=>!t.available)));});}
