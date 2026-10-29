import {expect,test} from 'vitest';
import {dropImportSource} from '../src/importDestination';
import type {Source} from '../src/types';
const source=(id:number,count:number,extra:Partial<Source>={}):Source=>({id,path:`/music/${id}`,display_name:String(id),available:true,enabled:true,song_count:count,last_error:null,...extra});
test('drops join the largest available enabled folder, with a stable tie break',()=>{
 const result=dropImportSource({tracks:[],sources:[source(1,3),source(2,500),source(3,900,{available:false}),source(4,800,{enabled:false}),source(5,500)]});expect(result?.id).toBe(2);
 expect(dropImportSource({tracks:[],sources:[source(1,0)]})).toBeUndefined();
});
