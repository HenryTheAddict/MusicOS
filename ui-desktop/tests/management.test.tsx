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
test('metadata comparison applies only chosen fields with the reviewed revision and token',async()=>{
 const suggestion={track_id:1,current_title:'Song',current_artist:'Artist',current_album:'Studio',current_year:2020,revision:7,candidate_token:'version-1',candidate:{title:'Song',artist:'Artist',album:'Live',year:2020,confidence:.8}};
 bridge.call.mockResolvedValue({items:[suggestion],total:1});const refresh=vi.fn().mockResolvedValue(undefined);render(<MetadataWorkspace library={{tracks:[],sources:[]} as unknown as Library} busy={false} onTask={()=>{}} onRefresh={refresh}/>);
 fireEvent.click(await screen.findByRole('button',{name:/Review Song by Artist/}));expect(screen.getByRole('checkbox',{name:'Apply album'})).toBeChecked();expect(screen.getByRole('checkbox',{name:'Apply title'})).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'Apply selected changes'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('metadata_workspace',{request:{action:'apply',trackId:1,revision:7,token:'version-1',fields:['album'],search:'',offset:0}}));expect(refresh).toHaveBeenCalled();
});
