import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,test,vi} from 'vitest';
import type {Library,SavedCollection} from '../src/types';
const bridge=vi.hoisted(()=>({call:vi.fn(),playlistFile:vi.fn()}));
vi.mock('../src/bridge',()=>({call:bridge.call,errorText:(error:unknown)=>String(error)}));
vi.mock('../src/components',()=>({Icon:()=>null,collectionDuration:(seconds:number)=>`${seconds}s`}));
vi.mock('../src/PlaylistLibrary',()=>({playlistFile:bridge.playlistFile,PlaylistArtwork:({item}:{item:SavedCollection})=><span data-testid="draft-artwork">{item.artwork_choice}:{item.track_ids.join(',')}</span>}));
vi.mock('../src/SongSelector',()=>({default:({onSelect,onConfirm}:{onSelect:(id:number)=>void;onConfirm:()=>void})=><div role="dialog" aria-label="Choose playlist songs"><button type="button" onClick={()=>onSelect(2)}>Second song</button><button type="button" onClick={()=>onSelect(1)}>First song</button><button type="button" onClick={onConfirm}>Use songs</button></div>}));
import CollectionCreate from '../src/CollectionDesign';
const library={tracks:[{id:1,title:'First song',duration:120},{id:2,title:'Second song',duration:180}]} as Library;
const props=()=>({kind:'manual' as const,library,items:[],onChange:vi.fn(),onOpen:vi.fn(),onClose:vi.fn()});
beforeEach(()=>{HTMLDialogElement.prototype.showModal=function(){this.setAttribute('open','');this.querySelector('button')?.focus();};HTMLDialogElement.prototype.close=function(){this.removeAttribute('open');};});
afterEach(()=>{cleanup();vi.clearAllMocks();});

test('live artwork and name retain the ordered song selection through artwork edits and save',async()=>{
 const handlers=props(),saved={id:'saved',name:'Evening',track_ids:[2,1],spark:null,artwork_choice:'playlist/Lake'};
 bridge.call.mockResolvedValue([saved]);render(<CollectionCreate {...handlers}/>);
 expect(screen.getByRole('textbox',{name:'Name'})).toHaveFocus();
 expect(screen.getByRole('button',{name:'Create playlist'})).toBeDisabled();
 fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:' Evening '}});
 expect(screen.getByText('Evening')).toBeVisible();
 fireEvent.click(screen.getByRole('button',{name:/Choose songs/}));fireEvent.click(screen.getByRole('button',{name:'Second song'}));fireEvent.click(screen.getByRole('button',{name:'First song'}));fireEvent.click(screen.getByRole('button',{name:'Use songs'}));
 expect(screen.getByText('2 songs · 300s')).toBeVisible();
 fireEvent.click(screen.getByRole('button',{name:'Change artwork'}));fireEvent.click(screen.getByRole('button',{name:'Choose Lake cover'}));
 expect(screen.getByTestId('draft-artwork')).toHaveTextContent('playlist/Lake:2,1');expect(screen.getByRole('textbox',{name:'Name'})).toHaveValue(' Evening ');
 fireEvent.click(screen.getByRole('button',{name:'Create playlist'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('collections',{request:{action:'create',name:'Evening',artwork_choice:'playlist/Lake',track_ids:[2,1]}}));
 expect(handlers.onChange).toHaveBeenCalledWith([saved]);expect(handlers.onOpen).toHaveBeenCalledWith(saved);expect(handlers.onClose).toHaveBeenCalledOnce();
});

test('an empty playlist is allowed, whitespace is not, and a failed save keeps the draft',async()=>{
 bridge.call.mockRejectedValueOnce(Error('Try again')).mockResolvedValueOnce([]);render(<CollectionCreate {...props()}/>);
 const input=screen.getByRole('textbox',{name:'Name'});fireEvent.change(input,{target:{value:'   '}});expect(screen.getByRole('button',{name:'Create playlist'})).toBeDisabled();
 fireEvent.change(input,{target:{value:'Quiet'}});fireEvent.click(screen.getByRole('button',{name:'Create playlist'}));await screen.findByRole('alert');expect(input).toHaveValue('Quiet');
 fireEvent.click(screen.getByRole('button',{name:'Create playlist'}));await waitFor(()=>expect(bridge.call).toHaveBeenCalledTimes(2));expect(bridge.call.mock.calls[1][1].request.track_ids).toEqual([]);
});

test('canceling import preserves the name and artwork draft',async()=>{
 bridge.playlistFile.mockResolvedValue(null);const handlers=props();render(<CollectionCreate {...handlers}/>);
 fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:'Keep this'}});fireEvent.click(screen.getByRole('button',{name:'Import M3U'}));
 await waitFor(()=>expect(screen.getByRole('button',{name:'Import M3U'})).toBeEnabled());expect(bridge.playlistFile).toHaveBeenCalledWith('import');expect(screen.getByRole('textbox',{name:'Name'})).toHaveValue('Keep this');expect(handlers.onClose).not.toHaveBeenCalled();expect(bridge.call).not.toHaveBeenCalled();
});
