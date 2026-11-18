import type {Playback} from './types';
/** Clock ticks bring fresh position and level data. Keep the existing queue
 * marks when they match, so the queue view can sit this update out. */
export function mergePlaybackUpdate(previous:Playback,update:Partial<Playback>):Playback{
 const next={...previous,...update};
 for(const key of ['queue_entry_ids','arranged_entries','pinned_entries'] as const){
  const value=update[key],old=previous[key];
  if(value&&old&&value.length===old.length&&value.every((id,index)=>id===old[index]))next[key]=old;
 }
 return next;
}

/** The next queue entry changed? That old mix prediction missed its invitation. */
export function queuedMixForCurrent(player: Playback){
 const index=player.queue_index??-1, entries=player.queue_entry_ids;
 if(index<0||!entries)return;
 let next=index+1;
 while(next<entries.length&&player.failed_entries?.includes(entries[next]))next++;
 return player.dj_queue_mixes?.find(mix=>mix.outgoing_entry===entries[index]&&mix.incoming_entry===entries[next]);
}
