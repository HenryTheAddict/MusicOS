import {expect,test} from 'vitest';
import {searchScore,groupMusicResults} from '../src/commandSearch';
test('exact release matches precede songs matched through their album or credits',()=>{
 const items=[{group:'Songs',title:'TEACUP EYES',score:searchScore('kazootopia','TEACUP EYES')},{group:'Albums',title:'KAZOOTOPIA',score:searchScore('kazootopia','KAZOOTOPIA')}];
 expect(groupMusicResults(items).map(item=>item.title)).toEqual(['KAZOOTOPIA','TEACUP EYES']);
 expect(searchScore('pink','PinkPuddleworks')).toBeLessThan(searchScore('pink','The Pink Album'));
});
test('groups prioritize their best match, retain stable ties and bound large libraries',()=>{
 const rows=Array.from({length:200},(_,i)=>({group:'Songs',title:`Song ${i}`,score:2}));
 expect(groupMusicResults(rows)).toHaveLength(12);
 expect(groupMusicResults([{group:'Albums',title:'Album',score:3},{group:'Artists',title:'Exact',score:0}])[0].group).toBe('Artists');
});
