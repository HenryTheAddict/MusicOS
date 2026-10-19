import {test,expect} from 'vitest';
import {mergePlaybackUpdate} from '../src/playbackPresentation';
import {emptyPlayback} from '../src/bridge';
test('clock updates retain queue and unchanged ordering marks without losing playback fields',()=>{
 const old={...emptyPlayback,queue_entry_ids:[1,2,3],pinned_entries:[2],arranged_entries:[3]};
 const next=mergePlaybackUpdate(old,{position:12,is_playing:true,pinned_entries:[2],arranged_entries:[3],queue_entry_ids:[1,2,3]});
 expect(next.position).toBe(12);expect(next.is_playing).toBe(true);expect(old.position).toBe(0);
 expect(next.queue).toBe(old.queue);expect(next.queue_entry_ids).toBe(old.queue_entry_ids);expect(next.pinned_entries).toBe(old.pinned_entries);expect(next.arranged_entries).toBe(old.arranged_entries);
});
test('edits retain duplicate entry identities and replace changed or explicitly cleared marks',()=>{
 const old={...emptyPlayback,queue_entry_ids:[11,12,13],pinned_entries:[12],arranged_entries:[13]};
 const ids=[13,11,12],next=mergePlaybackUpdate(old,{queue_entry_ids:ids,queue_index:1,pinned_entries:[],arranged_entries:[11]});
 expect(next.queue_entry_ids).toBe(ids);expect(next.queue_entry_ids).not.toBe(old.queue_entry_ids);expect(next.queue_index).toBe(1);expect(next.pinned_entries).toEqual([]);expect(next.arranged_entries).toEqual([11]);expect(old.pinned_entries).toEqual([12]);
});
