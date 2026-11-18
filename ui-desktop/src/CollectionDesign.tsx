import {vibeCover,parseVibe} from './vibeArtwork';
import {useEffect,useRef,useState,type ReactNode} from 'react';
import {call,errorText} from './bridge';
import {Icon,collectionDuration} from './components';
import SongSelector from './SongSelector';
import {playlistFile,PlaylistArtwork} from './PlaylistLibrary';
import type {Library,SavedCollection} from './types';
const files=import.meta.glob('../../assets/covers/playlists/*.png',{eager:true,query:'?url',import:'default'}) as Record<string,string>;
export const collectionCovers=Object.entries(files).map(([path,url])=>{const name=path.split('/').at(-1)!.slice(0,-4);return {id:name.startsWith('sparklist_')?`auto/${name.slice(10)}`:`playlist/${name}`,name:name.replaceAll('_',' '),url};}).filter(c=>!c.name.startsWith('pl ')).sort((a,b)=>a.id.localeCompare(b.id));
export function collectionCover(choice?:string|null){return vibeCover(choice)??collectionCovers.find(c=>c.id===choice)?.url;}
export function suggestedSparkCover(generated?:string|null){const art=parseVibe(generated);return art?`auto/${1+((art.seed+art.variant+Math.round(art.hue/40)+art.shape*3)%9)}`:'auto/1';}
export function CoverPicker({value,onChange,spark=false}:{value:string;onChange:(choice:string)=>void;spark?:boolean}){
 return <fieldset className="cover-picker"><legend>Cover artwork</legend><div className="cover-options">{value.startsWith("vibe/")&&<button type="button" aria-label="Use generated vibe cover" aria-pressed="true"><img src={vibeCover(value)} alt="Generated from this mix"/></button>}{collectionCovers.filter(c=>spark?c.id.startsWith('auto/'):c.id.startsWith('playlist/')).map(c=><button type="button" key={c.id} aria-label={`Choose ${c.name} cover`} aria-pressed={value===c.id} onClick={()=>onChange(c.id)}><img src={c.url} alt="" loading="lazy"/></button>)}</div><button type="button" className="collage-choice" aria-pressed={value==='collage'} onClick={()=>onChange('collage')}>Use album-cover collage</button></fieldset>;
}
export function CoverChoice({value,onChange,spark=false,preview,disabled=false}:{value:string;onChange:(choice:string)=>void;spark?:boolean;preview?:ReactNode;disabled?:boolean}){
 const [open,setOpen]=useState(false);return <><button type="button" className="cover-choice" aria-label="Change artwork" disabled={disabled} onClick={()=>setOpen(true)} aria-haspopup="dialog">{preview?<span className="cover-choice-preview">{preview}</span>:collectionCover(value)?<img src={collectionCover(value)} alt="Current cover"/>:<Icon name="Playlist"/>}<span>Change artwork</span></button>{open&&<ArtworkPicker value={value} spark={spark} onChange={choice=>{onChange(choice);setOpen(false);}} onClose={()=>setOpen(false)}/>}</>;
}
function ArtworkPicker({value,onChange,spark,onClose}:{value:string;onChange:(choice:string)=>void;spark:boolean;onClose:()=>void}){
 const ref=useRef<HTMLDialogElement>(null),opener=useRef(document.activeElement as HTMLElement|null);useEffect(()=>{ref.current?.showModal();return()=>{ref.current?.close();opener.current?.isConnected&&opener.current.focus();};},[]);
 return <dialog ref={ref} className="artwork-picker song-selector" aria-label="Choose cover artwork" onCancel={e=>{e.preventDefault();e.stopPropagation();onClose();}}><header><h2>Cover artwork</h2><button type="button" aria-label="Close artwork picker" onClick={onClose}>×</button></header><CoverPicker value={value} spark={spark} onChange={onChange}/></dialog>;
}
export function PuffyToggle<T extends string>({label,value,options,onChange}:{label:string;value:T;options:{value:T;label:string}[];onChange:(value:T)=>void}){
 return <div className="puffy-toggle" role="group" aria-label={label}>{options.map(option=><button key={option.value} aria-pressed={value===option.value} onClick={()=>onChange(option.value)}>{option.label}</button>)}</div>;
}
export default function CollectionCreate({kind,library,items,onChange,onOpen,onClose,sparkContent}:{kind:'manual'|'spark';library:Library;items:SavedCollection[];onChange:(items:SavedCollection[])=>void;onOpen:(item:SavedCollection)=>void;onClose:()=>void;sparkContent?:React.ReactNode}){
 const opener=useRef(document.activeElement as HTMLElement|null);
 const [songs,setSongs]=useState<number[]>([]),[choosingSongs,setChoosingSongs]=useState(false);
 const dialog=useRef<HTMLDialogElement>(null),nameInput=useRef<HTMLInputElement>(null);const [name,setName]=useState(''),[choice,setChoice]=useState('playlist/tritower'),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{const element=dialog.current;element?.showModal();nameInput.current?.focus({preventScroll:true});return()=>{element?.close();opener.current?.focus();};},[]);
 const selectedTracks=library.tracks.filter(track=>songs.includes(track.id));
 const draft:SavedCollection={id:'draft',name:name.trim()||'New playlist',track_ids:songs,spark:null,kind:'manual',artwork_choice:choice};
 async function create(){if(busy||!name.trim())return;setBusy(true);setError('');try{const next=await call('collections',{request:{action:'create',name:name.trim(),artwork_choice:choice,track_ids:songs}});onChange(next);const added=next.find(c=>!items.some(old=>old.id===c.id));if(added)onOpen(added);onClose();}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 async function importPlaylist(){if(busy)return;setBusy(true);setError('');try{const result=await playlistFile('import');if(result){onChange(result.items);if(result.missing.length)setError(`Imported ${result.imported} songs. Unmatched entries: ${result.missing.join(', ')}`);else{const added=result.items.find(c=>!items.some(old=>old.id===c.id));if(added)onOpen(added);onClose();}}}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <dialog ref={dialog} className={`collection-create ${kind==='spark'?'spark-create':''}`} aria-label={kind==='spark'?'Create Sparklist':'Create playlist'} onCancel={e=>{e.preventDefault();onClose();}} onClick={e=>{if(e.target===dialog.current){const r=dialog.current.getBoundingClientRect();if(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom)onClose();}}}>
  {kind==='spark'?sparkContent:<form className="playlist-composer" onSubmit={e=>{e.preventDefault();void create();}}>
   <header className="playlist-composer-heading"><span className="playlist-composer-mark"><Icon name="Playlist"/></span><h2>Create playlist</h2><button type="button" className="playlist-close" aria-label="Close playlist creator" onClick={onClose}><Icon name="controlClose"/></button></header>
   <div className="playlist-composer-body"><aside className="playlist-artwork-preview"><CoverChoice value={choice} onChange={setChoice} disabled={busy} preview={<PlaylistArtwork item={draft} library={library}/>}/><strong>{draft.name}</strong><small>{songs.length} {songs.length===1?'song':'songs'}{songs.length>0?` · ${collectionDuration(selectedTracks.reduce((sum,track)=>sum+track.duration,0))}`:''}</small></aside>
    <div className="playlist-composer-fields"><label>Name<input ref={nameInput} autoFocus required maxLength={120} disabled={busy} value={name} onChange={e=>setName(e.target.value)} placeholder="Give it a name"/></label><section><div className="playlist-songs-label"><h3>Songs</h3><span>Optional</span></div><button type="button" className="select-playlist-songs" disabled={busy} aria-haspopup="dialog" onClick={()=>setChoosingSongs(true)}><Icon name="Song"/><span><strong>{songs.length?`${songs.length} ${songs.length===1?'song':'songs'} selected`:'Choose songs'}</strong><small>{songs.length?'Edit selection':'Or add them later'}</small></span><Icon name="Plus"/></button></section></div>
   </div>
   {error&&<p className="playlist-create-error" role="alert">{error}</p>}
   <footer className="playlist-composer-footer"><button type="button" className="playlist-import" disabled={busy} onClick={()=>void importPlaylist()}><Icon name="File"/>Import M3U</button><div><button type="button" className="sketch-button secondary" onClick={onClose}>Cancel</button><button className="sketch-button" disabled={busy||!name.trim()}><Icon name="Playlist"/>{busy?'Creating…':'Create playlist'}</button></div></footer>
   {choosingSongs&&<SongSelector tracks={library.tracks} selected={null} multiple={songs} onSelect={id=>setSongs(old=>old.includes(id)?old.filter(value=>value!==id):[...old,id])} onConfirm={()=>setChoosingSongs(false)} onClose={()=>setChoosingSongs(false)}/>}
  </form>}
 </dialog>;
}
