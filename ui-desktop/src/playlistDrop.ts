import type {SavedCollection} from './types';
export function playlistDropPlan(item:SavedCollection|undefined,ids:number[],available:ReadonlySet<number>){
 if(!item)return {ids:[],label:'Choose a playlist',allowed:false};
 if(item.spark||item.kind==='rule'||item.kind==='audio')return {ids:[],label:'This collection manages its own songs',allowed:false};
 const incoming=[...new Set(ids)].filter(id=>available.has(id)),existing=new Set(item.track_ids),added=incoming.filter(id=>!existing.has(id));
 return {ids:added,allowed:added.length>0,label:added.length?`Add ${added.length} ${added.length===1?'song':'songs'} to ${item.name}${incoming.length>added.length?' · skip existing songs':''}`:incoming.length?`Already in ${item.name}`:'These songs are unavailable'};
}
