import {vibeCover,parseVibe} from './vibeArtwork';
import {useEffect,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import {Icon} from './components';
import {playlistFile} from './PlaylistLibrary';
import type {Library,SavedCollection} from './types';
const files=import.meta.glob('../../assets/covers/playlists/*.png',{eager:true,query:'?url',import:'default'}) as Record<string,string>;
export const collectionCovers=Object.entries(files).map(([path,url])=>{const name=path.split('/').at(-1)!.slice(0,-4);return {id:name.startsWith('sparklist_')?`auto/${name.slice(10)}`:`playlist/${name}`,name:name.replaceAll('_',' '),url};}).filter(c=>!c.name.startsWith('pl ')).sort((a,b)=>a.id.localeCompare(b.id));
export function collectionCover(choice?:string|null){return vibeCover(choice)??collectionCovers.find(c=>c.id===choice)?.url;}
export function suggestedSparkCover(generated?:string|null){const art=parseVibe(generated);return art?`auto/${1+((art.seed+art.variant+Math.round(art.hue/40)+art.shape*3)%9)}`:'auto/1';}
export function CoverPicker({value,onChange,spark=false}:{value:string;onChange:(choice:string)=>void;spark?:boolean}){
 return <fieldset className="cover-picker"><legend>Cover artwork</legend><div className="cover-options">{value.startsWith("vibe/")&&<button type="button" aria-label="Use generated vibe cover" aria-pressed="true"><img src={vibeCover(value)} alt="Generated from this mix"/></button>}{collectionCovers.filter(c=>spark?c.id.startsWith('auto/'):c.id.startsWith('playlist/')).map(c=><button type="button" key={c.id} aria-label={`Choose ${c.name} cover`} aria-pressed={value===c.id} onClick={()=>onChange(c.id)}><img src={c.url} alt="" loading="lazy"/></button>)}</div><button type="button" className="collage-choice" aria-pressed={value==='collage'} onClick={()=>onChange('collage')}>Use album-cover collage</button></fieldset>;
}
export function PuffyToggle<T extends string>({label,value,options,onChange}:{label:string;value:T;options:{value:T;label:string}[];onChange:(value:T)=>void}){
 return <div className="puffy-toggle" role="group" aria-label={label}>{options.map(option=><button key={option.value} aria-pressed={value===option.value} onClick={()=>onChange(option.value)}>{option.label}</button>)}</div>;
}
export default function CollectionCreate({kind,library,items,onChange,onOpen,onClose,sparkContent}:{kind:'manual'|'spark';library:Library;items:SavedCollection[];onChange:(items:SavedCollection[])=>void;onOpen:(item:SavedCollection)=>void;onClose:()=>void;sparkContent?:React.ReactNode}){
 const opener=useRef(document.activeElement as HTMLElement|null);
 const dialog=useRef<HTMLDialogElement>(null);const [name,setName]=useState(''),[choice,setChoice]=useState('playlist/tritower'),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{const element=dialog.current;element?.showModal();return()=>{element?.close();opener.current?.focus();};},[]);
 async function create(){if(busy)return;setBusy(true);setError('');try{const next=await call('collections',{request:{action:'create',name,artwork_choice:choice}});onChange(next);const added=next.find(c=>!items.some(old=>old.id===c.id));if(added)onOpen(added);onClose();}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 async function importPlaylist(){setBusy(true);try{const result=await playlistFile('import');if(result){onChange(result.items);if(result.missing.length)setError(`Imported ${result.imported} songs. Unmatched entries: ${result.missing.join(', ')}`);else{const added=result.items.find(c=>!items.some(old=>old.id===c.id));if(added)onOpen(added);onClose();}}}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <dialog ref={dialog} className={`collection-create ${kind==='spark'?'spark-create':''}`} aria-label={kind==='spark'?'Create Sparklist':'Create playlist'} onCancel={e=>{e.preventDefault();onClose();}} onClick={e=>{if(e.target===dialog.current){const r=dialog.current.getBoundingClientRect();if(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom)onClose();}}}>
  <Icon name={kind==='spark'?'Sparklist':'Playlist'}/>{kind==='spark'?sparkContent:<form onSubmit={e=>{e.preventDefault();void create();}}><h2>Your next playlist.</h2><label>Name<input autoFocus required maxLength={120} value={name} onChange={e=>setName(e.target.value)} placeholder="Give it a name"/></label><CoverPicker value={choice} onChange={setChoice}/><button className="sketch-button" disabled={busy}>Create playlist</button><button type="button" className="sketch-button secondary" disabled={busy} onClick={()=>void importPlaylist()}>Import M3U</button></form>}
  {error&&<p role="alert">{error}</p>}<button className="sketch-button secondary nevermind" onClick={onClose}>Nevermind.</button>
 </dialog>;
}
