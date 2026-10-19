import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import type {Library,SavedCollection,SparkState} from '../src/types';
const bridge=vi.hoisted(()=>({call:vi.fn()}));vi.mock('../src/bridge',()=>({call:bridge.call,errorText:(e:unknown)=>String(e),native:false}));vi.mock('../src/components',()=>({Cover:()=>null,Icon:()=>null,collectionDuration:(s:number)=>String(s),duration:(s:number)=>String(s)}));
import {SmartEditor} from '../src/SmartEditor';
import {collectionKind,filterCollections} from '../src/CollectionManager';
import MetadataWorkspace from '../src/MetadataWorkspace';
import {sparkSetupStatus} from '../src/SparkSetup';
const smart={id:'spark/1',name:'Rules',track_ids:[1],spark:{name:'Rules',match_mode:'any',limit:40,rules:[{field:'artist',op:'contains',value:'M83'},{field:'year',op:'greater_than',value:'2000'}]}} as SavedCollection;
afterEach(()=>{cleanup();bridge.call.mockReset();});
test('collection tabs distinguish audio and rules, preserve identities and sort recent changes',()=>{
 const manual={id:'playlist/1',name:'Zed',spark:null,track_ids:[],updated_at:5} as SavedCollection,audio={id:'audio/1',name:'Mix',spark:null,kind:'audio',track_ids:[],updated_at:10} as SavedCollection;
 expect([manual,audio,smart].map(collectionKind)).toEqual(['manual','audio','rule']);expect(filterCollections([manual,audio,smart],'audio','mix','recent')).toEqual([audio]);expect(filterCollections([manual,{...manual,id:'playlist/2',updated_at:20}],'manual','','recent')[0].id).toBe('playlist/2');
});
test('editing a Smart playlist name preserves every rule, match mode and limit',async()=>{
 bridge.call.mockResolvedValue([]);render(<SmartEditor item={smart} onChange={()=>{}} onClose={()=>{}}/>);
 fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:'Renamed'}});fireEvent.click(screen.getByRole('button',{name:'Save Smart playlist'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('collections',{request:{action:'update_spark',id:'spark/1',definition:{...smart.spark,name:'Renamed'}}}));
});
test('setup reports actionable states separately from recommendation quality',()=>{
 expect(sparkSetupStatus(false,null,'')).toBe('Beta off');expect(sparkSetupStatus(true,{ready:false} as SparkState,'')).toBe('Setup needed');expect(sparkSetupStatus(true,{ready:true,analyzed:1} as SparkState,'')).toBe('Preparing songs');expect(sparkSetupStatus(true,{ready:true,analyzed:10} as SparkState,'')).toBe('Ready');expect(sparkSetupStatus(true,null,'checksum')).toBe('Needs attention');
});
test('album comparison applies only changed fields with the reviewed album token',async()=>{
 const album={album_id:'album-1',title:'Studio',artist:'Artist',token:'version-1',rows:[{track_id:1,current:{title:'Song',artist:'Artist',album:'Studio',year:2020,revision:7},candidate_token:'candidate-1',candidate:{title:'Song',artist:'Artist',album:'Live',year:2020,confidence:.8}}]};
 bridge.call.mockResolvedValue({albums:[album],items:[],total:0});const refresh=vi.fn().mockResolvedValue(undefined);render(<MetadataWorkspace library={{tracks:[],albums:[],sources:[]} as unknown as Library} busy={false} onTask={()=>{}} onRefresh={refresh}/>);
 fireEvent.click(await screen.findByRole('button',{name:/Studio/}));expect(screen.getByRole('checkbox',{name:'Apply album'})).toBeChecked();expect(screen.queryByRole('checkbox',{name:'Apply title for Song'})).toBeNull();fireEvent.click(screen.getByRole('checkbox',{name:'Show unchanged track fields'}));expect(screen.getByRole('checkbox',{name:'Apply title for Song'})).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'Review changes'}));fireEvent.click(screen.getByRole('button',{name:'Apply reviewed changes'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('album_metadata',{request:{action:'apply',album_id:'album-1',token:'version-1',rows:[{track_id:1,fields:['album']}],write_files:false}}));expect(refresh).toHaveBeenCalled();
});

test('configured managed storage has no Forget action while empty legacy storage can be cleaned up',async()=>{
 const {default:LibraryTools}=await import('../src/LibraryTools');const library={tracks:[],albums:[],artists:[],genres:[],protected_source_ids:[1],sources:[{id:1,path:'/managed',display_name:'Current MusicOS',kind:'managed',song_count:0,enabled:true,available:true,last_error:null},{id:2,path:'/legacy',display_name:'Legacy MusicOS',kind:'managed',song_count:0,enabled:true,available:false,last_error:null}]} as Library;
 render(<LibraryTools view="sources" onError={()=>{}} library={library} busy={false} onAdd={()=>{}} onTask={()=>{}} onRefresh={async()=>{}}/>);expect(screen.getAllByRole('button',{name:'Forget…'})).toHaveLength(1);expect(screen.getByRole('heading',{name:'Current MusicOS'}).closest('article')?.querySelector('button')).toBeNull();
});
