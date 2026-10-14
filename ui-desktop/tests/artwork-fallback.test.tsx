import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({extract:vi.fn(),call:vi.fn()}));
vi.mock('../src/bridge',()=>({native:true,call:bridge.call,artwork:(path:string)=>path,asset:()=>'/placeholder.svg',libraryArtwork:bridge.extract,localArtwork:vi.fn()}));
import {registerAlbumArtwork} from '../src/albumArtwork';
import type {Library} from '../src/types';
import {Cover} from '../src/components';
afterEach(()=>{cleanup();vi.unstubAllGlobals();bridge.extract.mockReset();});
test('unreadable saved artwork retries embedded extraction and can reload the repaired cache path',async()=>{
 vi.stubGlobal('IntersectionObserver',class {callback:Function;constructor(callback:Function){this.callback=callback;}observe(){queueMicrotask(()=>this.callback([{isIntersecting:true}]));}disconnect(){}});
 bridge.extract.mockResolvedValue('/cached-cover.png');
 render(<Cover src="/cached-cover.png" trackId={7} title="Album"/>);
 expect(bridge.extract).not.toHaveBeenCalled();
 fireEvent.error(screen.getByRole('img'));
 await waitFor(()=>expect(bridge.extract).toHaveBeenCalledWith(7,false));
 await waitFor(()=>expect(screen.getByRole('img')).toHaveAttribute('src','/cached-cover.png?musicos-artwork=0'));
 expect(bridge.extract).toHaveBeenCalledTimes(1);
});
test('songs with no embedded artwork keep the placeholder without repeated extraction',async()=>{
 vi.stubGlobal('IntersectionObserver',class {callback:Function;constructor(callback:Function){this.callback=callback;}observe(){queueMicrotask(()=>this.callback([{isIntersecting:true}]));}disconnect(){}});
 bridge.extract.mockResolvedValue(null);
 render(<Cover src={null} trackId={8} title="No artwork"/>);
 await waitFor(()=>expect(bridge.extract).toHaveBeenCalledWith(8,false));
 expect(screen.getByRole('img')).toHaveAttribute('src','/placeholder.svg');expect(bridge.extract).toHaveBeenCalledTimes(1);
});

test('song artwork checks its own file even when a valid album cover is already present',async()=>{
 vi.stubGlobal('IntersectionObserver',class {callback:Function;constructor(callback:Function){this.callback=callback;}observe(){queueMicrotask(()=>this.callback([{isIntersecting:true}]));}disconnect(){}});
 bridge.extract.mockResolvedValue('/song-specific.png');
 render(<Cover songArtwork src="/shared-album.png" trackId={9} title="Song"/>);
 await waitFor(()=>expect(bridge.extract).toHaveBeenCalledWith(9,true));
 await waitFor(()=>expect(screen.getByRole('img')).toHaveAttribute('src','/song-specific.png?musicos-artwork=0'));
 expect(bridge.extract).toHaveBeenCalledTimes(1);
});

test('artist portraits bypass canonical album artwork and use local face framing',async()=>{
 registerAlbumArtwork({albums:[{cover:'/album.png',track_ids:[99]}]} as Library);bridge.call.mockResolvedValue('/face.png');
 render(<Cover portrait src="/portrait.png" trackId={99} title="Artist"/>);
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('portrait_artwork',{path:'/portrait.png'}));await waitFor(()=>expect(screen.getByRole('img')).toHaveAttribute('src','/face.png'));
 registerAlbumArtwork({albums:[]} as unknown as Library);
});
