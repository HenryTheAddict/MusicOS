import {expect,it} from 'vitest';
import {playlistDropPlan} from '../src/playlistDrop';
import type {SavedCollection} from '../src/types';
const playlist:SavedCollection={id:'p',name:'Night drive',kind:'manual',spark:null,track_ids:[2]};
it('adds only available new songs in the supplied order, once',()=>{
 const plan=playlistDropPlan(playlist,[3,2,1,3,99],new Set([1,2,3]));expect(plan.ids).toEqual([3,1]);expect(plan.allowed).toBe(true);expect(plan.label).toContain('skip existing');
});
it('explains duplicate-only and unavailable drops without a write',()=>{
 expect(playlistDropPlan(playlist,[2,2],new Set([2]))).toMatchObject({ids:[],allowed:false,label:'Already in Night drive'});expect(playlistDropPlan(playlist,[99],new Set([2])).allowed).toBe(false);
});
it('never edits generated or rule-owned collections through a drop',()=>{
 for(const kind of ['audio','rule'] as const)expect(playlistDropPlan({...playlist,kind},[1],new Set([1])).allowed).toBe(false);
 expect(playlistDropPlan({...playlist,kind:undefined,spark:{name:'Rule',match_mode:'all',rules:[],limit:null}},[1],new Set([1])).allowed).toBe(false);
});
