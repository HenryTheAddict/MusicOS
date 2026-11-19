import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import type {Library,SavedCollection,SparkState} from '../src/types';
const bridge=vi.hoisted(()=>({call:vi.fn()}));vi.mock('../src/bridge',()=>({call:bridge.call,errorText:(e:unknown)=>String(e),native:false}));vi.mock('../src/components',()=>({Cover:()=>null,Icon:()=>null,Menu:()=>null,collectionDuration:(s:number)=>String(s),duration:(s:number)=>String(s)}));
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
test('album browser reads a summary and loads only the selected comparison',async()=>{
 const album={album_id:'album-1',title:'Studio',artist:'Artist',token:'version-1',rows:[{track_id:1,current:{title:'Song',artist:'Artist',album:'Studio',year:2020,revision:7},candidate_token:'candidate-1',candidate:{title:'Song',artist:'Artist',album:'Live',year:2020,confidence:.8}}]};
 bridge.call.mockImplementation(async (_command,args)=>{
  if(args.request?.action==='summary')return {album_ids:['album-1']};
  if(args.request?.action==='detail')return {album};
  return {count:1};
 });
 const refresh=vi.fn().mockResolvedValue(undefined);
 const library={tracks:[],albums:[{id:'album-1',title:'Studio',artist:'Artist',track_ids:[1]},{id:'other',title:'Other',artist:'Artist',track_ids:[2]}],sources:[]} as unknown as Library;
 render(<MetadataWorkspace library={library} busy={false} onTask={()=>{}} onRefresh={refresh}/>);
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('album_metadata',{request:{action:'summary'}}));
 expect(bridge.call.mock.calls.every(([,args])=>args.request.action==='summary')).toBe(true);
 fireEvent.click(screen.getByRole('button',{name:/Studio/}));
 fireEvent.click(screen.getByRole('tab',{name:'Details'}));
 const change=await screen.findByRole('checkbox',{name:'Apply album for Song'});
 expect(change).not.toBeChecked();
 fireEvent.click(change);
 fireEvent.click(screen.getByRole('button',{name:'Review selected suggestions'}));
 fireEvent.click(screen.getByRole('button',{name:'Save in musicOS'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('album_metadata',{request:{action:'apply',album_id:'album-1',token:'version-1',rows:[{track_id:1,fields:['album']}],write_files:false}}));
 expect(refresh).toHaveBeenCalled();
 expect(screen.getByRole('heading',{name:'Studio'})).toBeVisible();
});

test('configured managed storage has no Forget action while empty legacy storage can be cleaned up',async()=>{
 const {default:LibraryTools}=await import('../src/LibraryTools');const library={tracks:[],albums:[],artists:[],genres:[],protected_source_ids:[1],sources:[{id:1,path:'/managed',display_name:'Current MusicOS',kind:'managed',song_count:0,enabled:true,available:true,last_error:null},{id:2,path:'/legacy',display_name:'Legacy MusicOS',kind:'managed',song_count:0,enabled:true,available:false,last_error:null}]} as Library;
 render(<LibraryTools view="sources" onError={()=>{}} library={library} busy={false} onAdd={()=>{}} onTask={()=>{}} onRefresh={async()=>{}} copyOnlyNonLibraryDrops={false} onCopyOnlyNonLibraryDropsChange={()=>{}}/>);expect(screen.getAllByRole('button',{name:'Forget…'})).toHaveLength(1);expect(screen.getByRole('heading',{name:'Current MusicOS'}).closest('article')?.querySelector('button')).toBeNull();
});

test('dropped music exposes a controlled switch for skipping library files',async()=>{
 const {default:LibraryTools}=await import('../src/LibraryTools');
 const props={view:'sources' as const,library:{tracks:[],albums:[],artists:[],genres:[],sources:[]},busy:false,onError:()=>{},onAdd:()=>{},onTask:()=>{},onRefresh:async()=>{},onCopyOnlyNonLibraryDropsChange:vi.fn()};
 const {rerender}=render(<LibraryTools {...props} copyOnlyNonLibraryDrops={false}/>);
 const toggle=screen.getByRole('switch',{name:'Only copy songs outside library folders'});
 expect(toggle).not.toBeChecked();
 fireEvent.click(toggle);
 expect(props.onCopyOnlyNonLibraryDropsChange).toHaveBeenCalledWith(true);
 rerender(<LibraryTools {...props} copyOnlyNonLibraryDrops/>);
 expect(toggle).toBeChecked();
});

test('cancel stops the explicit suggestion task and keeps cached suggestions available',async()=>{
 bridge.call.mockImplementation(async (command)=>command==='album_metadata'?{album_ids:['album-1']}:undefined);
 const library={tracks:[],albums:[{id:'album-1',title:'Studio',artist:'Artist',track_ids:[1]}],sources:[]} as unknown as Library;
 const props={library,busy:true,onTask:vi.fn(),onRefresh:async()=>{}};
 const task={kind:'enrich',running:true,processed:2,total:8,error:null,cancelled:false,message:''};
 const workspace=render(<MetadataWorkspace {...props} task={task}/>);
 expect(screen.getByRole('progressbar',{name:'Checking for suggestions'})).toHaveValue(2);
 fireEvent.click(screen.getByRole('button',{name:'Cancel check'}));
 expect(bridge.call).toHaveBeenCalledWith('cancel_task');
 workspace.rerender(<MetadataWorkspace {...props} busy={false} task={{...task,running:false,cancelled:true}}/>);
 expect(screen.getByText(/Check cancelled/)).toBeVisible();
 fireEvent.click(screen.getByRole('button',{name:'Has suggestions'}));
 expect(await screen.findByRole('button',{name:/Studio.*Suggestions available/})).toBeVisible();
 expect(props.onTask).not.toHaveBeenCalled();
});
