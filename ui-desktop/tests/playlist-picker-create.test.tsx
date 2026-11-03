import {cleanup,render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import PlaylistPicker from '../src/PlaylistPicker';
import type {Library,Track} from '../src/types';
const call=vi.hoisted(()=>vi.fn());
vi.mock('../src/bridge',()=>({call,errorText:String}));
vi.mock('../src/PlaylistLibrary',()=>({PlaylistArtwork:()=>null}));
vi.mock('../src/components',()=>({Icon:()=>null}));
beforeEach(()=>{call.mockReset();HTMLDialogElement.prototype.showModal=function(){this.open=true;};HTMLDialogElement.prototype.close=function(){this.open=false;};});
afterEach(cleanup);
it('creates a playlist containing the selected sequence in one acknowledged operation',async()=>{
 const created=[{id:'new',name:'Evening',track_ids:[3,1,3]}],change=vi.fn(),close=vi.fn();call.mockResolvedValue(created);
 render(<PlaylistPicker track={{id:3,title:'A song'} as Track} ids={[3,1,3]} items={[]} library={{tracks:[],albums:[],artists:[],genres:[],sources:[]} as Library} onChange={change} onClose={close}/>);
 const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'New playlist'}));
 expect(screen.getByRole('button',{name:'Create & add songs'})).toBeDisabled();await user.type(screen.getByRole('textbox',{name:'Playlist name'}),'  Evening  ');await user.click(screen.getByRole('button',{name:'Create & add songs'}));
 await waitFor(()=>expect(close).toHaveBeenCalledTimes(1));expect(call).toHaveBeenCalledExactlyOnceWith('collections',{request:{action:'create',name:'Evening',track_ids:[3,1,3],artwork_choice:'collage'}});expect(change).toHaveBeenCalledWith(created);
});
it('keeps the name and selected songs available when saving fails',async()=>{
 call.mockRejectedValueOnce(Error('Disk is busy')).mockResolvedValueOnce([]);const close=vi.fn();
 render(<PlaylistPicker track={{id:7,title:'A song'} as Track} items={[]} library={{tracks:[]} as unknown as Library} onChange={()=>{}} onClose={close}/>);
 const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'New playlist'}));await user.type(screen.getByRole('textbox',{name:'Playlist name'}),'Night');await user.click(screen.getByRole('button',{name:'Create & add songs'}));
 expect(await screen.findByRole('alert')).toHaveTextContent('Disk is busy');expect(close).not.toHaveBeenCalled();expect(screen.getByRole('textbox',{name:'Playlist name'})).toHaveValue('Night');await user.click(screen.getByRole('button',{name:'Create & add songs'}));await waitFor(()=>expect(close).toHaveBeenCalledTimes(1));
});
