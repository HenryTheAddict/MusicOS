import type {Playback} from './types';
/** Native position/level ticks omit queue membership, but include these marks.
 * Keep unchanged mark identities so clock updates do not rebuild queue views. */
export function mergePlaybackUpdate(previous:Playback,update:Partial<Playback>):Playback{
 const next={...previous,...update};
 for(const key of ['queue_entry_ids','arranged_entries','pinned_entries'] as const){
  const value=update[key],old=previous[key];
  if(value&&old&&value.length===old.length&&value.every((id,index)=>id===old[index]))next[key]=old;
 }
 return next;
}
