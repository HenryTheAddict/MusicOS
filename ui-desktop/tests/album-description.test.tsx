import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({call:vi.fn()}));vi.mock('../src/bridge',()=>({call:bridge.call}));
import AlbumDescription from '../src/AlbumDescription';
beforeEach(()=>{HTMLDialogElement.prototype.showModal=vi.fn(function(this:HTMLDialogElement){this.open=true;});HTMLDialogElement.prototype.close=vi.fn(function(this:HTMLDialogElement){this.open=false;});});
afterEach(()=>{cleanup();bridge.call.mockReset();vi.restoreAllMocks();});
test('available descriptions expand and keep their source attribution',async()=>{
 vi.spyOn(HTMLElement.prototype,'scrollHeight','get').mockReturnValue(200);
 vi.spyOn(HTMLElement.prototype,'clientHeight','get').mockReturnValue(100);
 bridge.call.mockResolvedValue({description:'An attributed album description.',source_url:'https://en.wikipedia.org/wiki/Test_album'});
 render(<AlbumDescription albumId="local/album/test"/>);await screen.findByText('An attributed album description.');
 fireEvent.click(await screen.findByRole('button',{name:'Read more'}));expect(screen.getByRole('dialog',{name:'About this album'})).toBeVisible();
 fireEvent.click(screen.getByRole('dialog').querySelector(':scope>button')!);expect(bridge.call).toHaveBeenLastCalledWith('open_provider',{url:'https://en.wikipedia.org/wiki/Test_album'});
});
test('short descriptions do not offer an empty expansion',async()=>{
 bridge.call.mockResolvedValue({description:'A short description.',source_url:null});
 render(<AlbumDescription albumId="short"/>);await screen.findByText('A short description.');
 expect(screen.queryByRole('button',{name:'Read more'})).not.toBeInTheDocument();
});
test('an unverified or unavailable description adds no invented content',async()=>{
 bridge.call.mockResolvedValue({description:null,source_url:null});const {container}=render(<AlbumDescription albumId="local/album/unknown"/>);
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('album_info',{albumId:'local/album/unknown'}));expect(container).toBeEmptyDOMElement();
});
