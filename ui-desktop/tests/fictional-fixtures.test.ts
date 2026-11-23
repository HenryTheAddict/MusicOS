import {expect,test,vi} from 'vitest';
import type {Library} from '../src/types';

test('demo library retains its identities and complete abstract artwork',async()=>{
  vi.resetModules();
  window.history.replaceState({},'', '/?fixture=1');
  const {fixtureCall}=await import('../src/fixtures');
  const {library}=await fixtureCall('bootstrap',{}) as {library:Library};
  expect(library.tracks).toHaveLength(144);
  expect(library.albums).toHaveLength(12);
  expect(library.artists).toHaveLength(7);
  expect(library.albums[0]).toMatchObject({id:'local/album/0',title:'Bubblewrap Lagoon',artist:'Velvet Static',track_ids:Array.from({length:12},(_,i)=>i+1)});
  expect(library.artists[3]).toMatchObject({id:'local/artist/3',name:'Moonspindle'});
  expect(library.artists[3].portrait).toContain('moonspindle.png');
  expect(library.albums[9].cover).toContain('sock-rocket.png');
  for(const album of library.albums)expect(album.cover).toMatch(/\.png/);
  for(const track of library.tracks){
    expect(track.stable_id).toBe(`fixture-${track.id}`);
    expect(track.cover_art_path).toBe(library.albums[Math.floor((track.id-1)/12)].cover);
  }
});
