import {expect,test} from 'vitest';
import {homeMusic} from '../src/homeData';
import type {Library} from '../src/types';
const library={tracks:[{id:1,available:true},{id:2,available:true},{id:3,available:false}],albums:[{id:'original',title:'Record',track_ids:[1]},{id:'edition',title:'Record',track_ids:[2]},{id:'missing',title:'Missing',track_ids:[3]}]} as Library;
test('home ranking preserves editions, sums actual counts, and filters unavailable history',()=>{
 const result=homeMusic(library,{frequent:[{id:1,plays:3},{id:2,plays:12},{id:3,plays:100},{id:999,plays:99}],recent:[3,999,1,1,2]});expect(result.rotation.map(a=>a.id)).toEqual(['edition','original']);expect(result.recent.map(t=>t.id)).toEqual([1,2]);
});
test('cold starts offer the library without inventing frequent plays or recent history',()=>{const result=homeMusic(library,undefined);expect(result.rotation).toEqual([]);expect(result.recent).toEqual([]);expect(result.explore.map(a=>a.id)).toEqual(['original','edition']);expect(homeMusic({...library,tracks:[]},undefined).explore).toEqual([]);});
