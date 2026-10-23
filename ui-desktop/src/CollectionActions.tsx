import {useEffect,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import type {SavedCollection} from './types';
export function CollectionDelete({item,onChange,onClose}:{item:SavedCollection;onChange:(items:SavedCollection[])=>void;onClose:()=>void}){
 const ref=useRef<HTMLDialogElement>(null),opener=useRef(document.activeElement as HTMLElement|null);const [busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{ref.current?.showModal();return()=>{ref.current?.close();if(opener.current?.isConnected)opener.current.focus();};},[]);
 return <dialog ref={ref} className="collection-create" aria-label={`Delete ${item.name}?`} onCancel={e=>{e.preventDefault();if(!busy)onClose();}}><h2>Delete {item.name}?</h2><p>Remove this collection? Music files stay in your library.</p>{error&&<p role="alert">{error}</p>}<button disabled={busy} onClick={async()=>{if(busy)return;setBusy(true);try{onChange(await call('collections',{request:{action:'delete',id:item.id}}));onClose();}catch(e){setError(errorText(e));setBusy(false);}}}>Delete collection</button><button disabled={busy} onClick={onClose}>Cancel</button></dialog>;
}
