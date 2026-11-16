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

/** A prediction becomes stale if the adjacent queue occurrence changes. */
export function queuedMixForCurrent(player: Playback){
 const index=player.queue_index??-1, entries=player.queue_entry_ids;
 if(index<0||!entries)return;
 let next=index+1;
 while(next<entries.length&&player.failed_entries?.includes(entries[next]))next++;
 return player.dj_queue_mixes?.find(mix=>mix.outgoing_entry===entries[index]&&mix.incoming_entry===entries[next]);
}
