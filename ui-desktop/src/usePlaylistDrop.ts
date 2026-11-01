import {haptic} from './haptics';
import {useEffect,useMemo,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import {playlistDropPlan} from './playlistDrop';
import {songDragFeedback,type SongDrag} from './songDrag';
import type {SavedCollection,Track} from './types';
export function usePlaylistDrop(collections:SavedCollection[],tracks:Track[],onChange:(items:SavedCollection[])=>void,onError:(message:string)=>void){
 const available=useMemo(()=>new Set(tracks.filter(t=>t.available).map(t=>t.id)),[tracks]);
 const latest=useRef({collections,available,onChange,onError});latest.current={collections,available,onChange,onError};
 const [notice,setNotice]=useState('');
 useEffect(()=>{if(!notice)return;const timer=setTimeout(()=>setNotice(''),4500);return()=>clearTimeout(timer);},[notice]);
 useEffect(()=>{let alive=true;const pending=new Set<string>();
  const clear=()=>document.querySelectorAll('[data-manual-collection][data-song-drop],[data-collection-target][data-song-drop]').forEach(el=>el.removeAttribute('data-song-drop'));
  const drag=(event:Event)=>{const value=(event as CustomEvent<SongDrag>).detail;clear();if(value.cancelled)return;const target=document.elementFromPoint(value.x,value.y)?.closest<HTMLElement>('[data-manual-collection],[data-collection-target]');if(!target)return;
   const id=target.dataset.manualCollection??target.dataset.collectionTarget!,item=latest.current.collections.find(item=>item.id===id),plan=playlistDropPlan(item,value.ids??[value.id],latest.current.available);
   if(pending.has(id)){songDragFeedback('Saving this playlist…',false);return;}
   songDragFeedback(plan.label,plan.allowed);if(!value.drop){if(plan.allowed)target.dataset.songDrop='true';return;}
   if(!plan.allowed){setNotice(plan.label);return;}pending.add(id);
   void call('collections',{request:{action:'add',id,track_ids:plan.ids}}).then(items=>{if(alive){latest.current.onChange(items);haptic('drop',id);setNotice(`Added ${plan.ids.length} ${plan.ids.length===1?'song':'songs'} to ${item!.name}`);}}).catch(error=>{if(alive)latest.current.onError(errorText(error));}).finally(()=>pending.delete(id));
  };
  window.addEventListener('musicos-song-drag',drag);return()=>{alive=false;clear();window.removeEventListener('musicos-song-drag',drag);};
 },[]);return notice;
}
