import {test,expect} from '@playwright/test';
import {orderedAlbums,albumTrackOrder} from '../ui/libraryOrder';
import type {Track} from '../../ui-desktop/src/types';
const track=(id:number,album_id:string,disc_number:number,track_number:number,title:string)=>({id,album_id,album:'Same title',artist:'Artist',year:2026,disc_number,track_number,title}) as Track&{album_id:string};
test('duplicate releases stay separate, unknown track numbers sort last, and source arrays stay intact',()=>{
 const original=[track(1,'release-a',2,1,'Disc two'),track(2,'release-a',1,0,'Unknown'),track(3,'release-b',1,1,'Other edition'),track(4,'release-a',1,2,'Second'),track(5,'release-a',1,1,'First')];
 const groups=orderedAlbums(original);
 expect(groups).toHaveLength(2);expect(groups.find(g=>g[0].album_id==='release-a')!.map(t=>t.id)).toEqual([5,4,2,1]);
 expect(original.map(t=>t.id)).toEqual([1,2,3,4,5]);
 expect([...original.slice(0,2)].sort(albumTrackOrder).map(t=>t.id)).toEqual([2,1]);
});
