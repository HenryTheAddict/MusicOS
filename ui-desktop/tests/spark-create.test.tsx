import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,test,vi} from 'vitest';
import type {Library,SparkState} from '../src/types';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({call:bridge.call,errorText:(e:unknown)=>String(e)}));
vi.mock('../src/components',()=>({Cover:()=>null,Icon:()=>null,Menu:({items,label}:{items:{label:string;disabled?:boolean;onSelect:()=>void}[];label:string})=><div>{items.map(item=><button key={item.label} disabled={item.disabled} aria-label={`${item.label} ${label.replace('Refine ','')}`} onClick={item.onSelect}>{item.label}</button>)}</div>,duration:(seconds:number)=>`${seconds}s`,collectionDuration:(seconds:number)=>`${seconds}s`}));
vi.mock('../src/CollectionDesign',()=>({CoverPicker:()=>null,CoverChoice:()=>null,collectionCover:()=>undefined,suggestedSparkCover:()=>"auto/1"}));
import SparkCreate from '../src/SparkCreate';
const library={tracks:[{id:1,title:'Seed song',artist:'Artist A',available:true,duration:120},{id:2,title:'Second song',artist:'Artist B',available:true,duration:180},{id:3,title:'Not yet analyzed',artist:'Artist C',available:true,duration:150}]} as Library;
const base:SparkState={ready:true,reason:null,download_bytes:277183168,analyzed:2,available:3,analyzed_ids:[1,2],preview:null,items:[],chosen_id:null};
const props=()=>({library,items:[],onChange:vi.fn(),onOpen:vi.fn(),onClose:vi.fn(),onPlay:vi.fn(),onSetup:vi.fn()});
beforeEach(()=>{HTMLElement.prototype.scrollTo=vi.fn();HTMLDialogElement.prototype.showModal=function(){this.setAttribute("open","");};HTMLDialogElement.prototype.close=function(){this.removeAttribute("open");};});
afterEach(()=>{cleanup();bridge.call.mockReset();});
test('unapproved model stays unavailable and never sends a generation request',async()=>{
 bridge.call.mockResolvedValue({...base,ready:false,reason:'Awaiting evaluation'});
 render(<SparkCreate {...props()}/>);await screen.findByText('Prepare your music to make Sparklists.');
 expect(screen.getByRole('button',{name:'Preview Sparklist'})).toBeDisabled();expect(screen.getByRole('button',{name:'Choose for me'})).toBeDisabled();
 expect(bridge.call).toHaveBeenCalledTimes(1);
});
test('motion timing follows the audible song BPM, then the seed, and never invents missing tempo',async()=>{
 bridge.call.mockResolvedValue({...base,ready:false});
 const shared=props(),tempos=new Map([[1,120],[2,90]]);
 const view=render(<SparkCreate {...shared} initialSeed={1} tempos={tempos} playingId={2} isPlaying/>);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeDisabled());
 const root=view.container.querySelector<HTMLElement>('.spark-create-content')!;
 expect(root).toHaveAttribute('data-bpm','90');expect(root.style.getPropertyValue('--spark-beat')).toBe(`${60/90}s`);
 view.rerender(<SparkCreate {...shared} initialSeed={1} tempos={tempos} playingId={2} isPlaying={false}/>);
 expect(root).toHaveAttribute('data-bpm','120');expect(root.style.getPropertyValue('--spark-beat')).toBe('0.5s');
 for(const missing of [undefined,NaN,Infinity,0,-30,401]){
  view.rerender(<SparkCreate {...shared} initialSeed={1} tempos={missing===undefined?new Map():new Map([[1,missing]])}/>);
  expect(root).toHaveAttribute('data-tempo-source','ambient');expect(root).not.toHaveAttribute('data-bpm');expect(root.style.getPropertyValue('--spark-beat')).toBe('0.6s');
 }
 view.rerender(<SparkCreate {...shared} initialSeed={1} tempos={tempos} playingId={3} isPlaying/>);
 expect(root).toHaveAttribute('data-tempo-source','ambient');
 expect(shared.onPlay).not.toHaveBeenCalled();
});
test('sparkles stay in the background and primary actions carry the shine, never the artwork',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:request.action==='choose'?{...base,chosen_id:1}:{...base,preview:{track_ids:[1,2],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 const view=render(<SparkCreate {...props()}/>);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 expect(view.container.querySelectorAll('.spark-glitter')).toHaveLength(1);
 expect(view.container.querySelectorAll('.spark-glitter>span')).toHaveLength(48);
 expect(view.container.querySelector('.spark-glitter')?.parentElement).toHaveClass('spark-composer-atmosphere');
 expect(view.container.querySelector('.spark-composer-atmosphere')).toHaveAttribute('aria-hidden','true');
 expect(view.container.querySelectorAll('.spark-artwork-sleeve .spark-glitter,.spark-composer-mark .spark-glitter')).toHaveLength(0);
 expect(screen.getByRole('button',{name:'Preview Sparklist'})).toHaveClass('spark-advance');
 expect(screen.getByRole('button',{name:'Choose for me'})).not.toHaveClass('spark-advance');
 fireEvent.click(screen.getByRole('button',{name:'Choose for me'}));
 const advance=await screen.findByRole('button',{name:'Name & save'});expect(advance).toHaveClass('spark-advance');fireEvent.click(advance);
 expect(screen.getByRole('button',{name:'Save Sparklist'})).toHaveClass('spark-advance');
 expect(view.container.querySelectorAll('.spark-artwork-sleeve .spark-glitter')).toHaveLength(0);
});
test('selected seed stays visible after filtering and unanalyzed seeds lead to Audio settings',async()=>{
 bridge.call.mockResolvedValue(base);const handlers=props();render(<SparkCreate {...handlers}/>);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/Not yet analyzed/}));
 fireEvent.click(screen.getByRole('button',{name:'Add starting song'}));fireEvent.change(screen.getByRole('searchbox',{name:'Filter songs'}),{target:{value:'Second'}});fireEvent.click(screen.getByRole('button',{name:'Close song selector'}));
 expect(screen.getByText('Starting from')).toBeVisible();expect(screen.getAllByText('Not yet analyzed').some(node=>node.closest('.spark-seed-toggle'))).toBe(true);
 expect(screen.getByRole('button',{name:'Preview Sparklist'})).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'Analyze starting song'}));await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('audio_analysis',{action:'analyze_seed',ids:[3]}));
 fireEvent.click(screen.getByRole('button',{name:'Clear starting song'}));expect(screen.queryByText('Starting from')).not.toBeInTheDocument();
});
test('preview listening is explicit and feedback remains part of the generation',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:{...base,preview:{track_ids:[1,2],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 const handlers=props();render(<SparkCreate {...handlers}/>);await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/Seed song/}));fireEvent.click(screen.getByRole('button',{name:'Dreamy'}));fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));
 await screen.findByText(/This mix uses 2 of 3/);expect(handlers.onPlay).not.toHaveBeenCalled();
 fireEvent.click(screen.getByRole('button',{name:'Listen to preview'}));expect(handlers.onPlay).toHaveBeenCalledWith([1,2]);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Listen to preview'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Less like this Second song'}));
 await waitFor(()=>expect(bridge.call).toHaveBeenLastCalledWith('sparklist',{request:{action:'preview',generation:{seed:1,mood:'Dreamy',less_like:[2],variant:1,target_count:25,refinement:{seed_ids:[1],moods:['Dreamy'],variety:50,discovery:50,kept:[],excluded_ids:[],more_like:[]}}}}));
});

test('creation picks up model and analysis readiness without remounting',async()=>{
 let ready=false;bridge.call.mockImplementation(async()=>ready?base:{...base,ready:false,beta:true,analyzed:0,analyzed_ids:[]});
 render(<SparkCreate {...props()}/>);await screen.findByText('Prepare your music to make Sparklists.');ready=true;
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled(),{timeout:2500});
 expect(screen.queryByText(/cached audio embeddings/)).not.toBeInTheDocument();
});
test('creation separates choosing, previewing and saving without losing the seed',async()=>{
 bridge.call.mockImplementation((_name:string,args:{request:{action:string;generation?:unknown}})=>Promise.resolve(args.request.action==='preview'?{...base,preview:{track_ids:[1,2],configuration:args.request.generation,analyzed:2,available:3,generation_version:'audio-cosine-mood-diversity-v2'}}:base));
 render(<SparkCreate {...props()}/>);await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/Seed song/}));fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));await screen.findByRole('button',{name:'Listen to preview'});expect(screen.queryByRole('textbox',{name:'Name'})).not.toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'Name & save'}));expect(screen.getByRole('textbox',{name:'Name'})).toBeVisible();fireEvent.click(screen.getByRole('button',{name:'Back to preview'}));fireEvent.click(screen.getByRole('button',{name:'Edit selection'}));expect(screen.getByText('Starting from')).toBeVisible();expect(screen.getAllByText('Seed song').some(el=>el.closest('.spark-seed-toggle'))).toBe(true);
});

 test('Choose for me prepares a preview from the chosen seed without starting playback',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:request.action==='choose'?{...base,chosen_id:2}:{...base,preview:{track_ids:[2,1],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 const handlers=props();render(<SparkCreate {...handlers}/>);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 fireEvent.click(screen.getByRole('button',{name:'Choose for me'}));
 await screen.findByRole('button',{name:'Listen to preview'});
 expect(bridge.call).toHaveBeenLastCalledWith('sparklist',{request:{action:'preview',generation:{seed:2,mood:null,less_like:[],variant:0,target_count:25,refinement:{seed_ids:[2],moods:[],variety:50,discovery:50,kept:[],excluded_ids:[],more_like:[]}}}});
 expect(handlers.onPlay).not.toHaveBeenCalled();
 });

test('name suggestions use the mood and seed without overwriting an edited name',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:{...base,preview:{track_ids:[1,2],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 render(<SparkCreate {...props()} library={{...library,tracks:library.tracks.map(t=>t.id===1?{...t,title:'November'}:t)}}/>);
 await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/November/}));fireEvent.click(screen.getByRole('button',{name:'Melancholic'}));fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));
 fireEvent.click(await screen.findByRole('button',{name:'Name & save'}));expect(screen.getByRole('textbox',{name:'Name'})).toHaveValue('');expect(screen.getByRole('button',{name:'November-esque'})).toBeVisible();
 fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:'My name'}});fireEvent.click(screen.getByRole('button',{name:'Back to preview'}));fireEvent.click(screen.getByRole('button',{name:'Name & save'}));expect(screen.getByRole('textbox',{name:'Name'})).toHaveValue('My name');
});

test('preview failures stay inside creation and can be retried',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:request.action==='choose'?{...base,chosen_id:2}:{...base,preview:{track_ids:[2,1],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 const handlers=props();handlers.onPlay.mockRejectedValueOnce(Error('File is unavailable')).mockResolvedValue(undefined);render(<SparkCreate {...handlers}/>);await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Choose for me'}));fireEvent.click(await screen.findByRole('button',{name:'Listen to preview'}));await screen.findByText(/Could not play this preview/);await waitFor(()=>expect(screen.getByRole('button',{name:'Listen to preview'})).toBeEnabled());fireEvent.click(screen.getByRole('button',{name:'Listen to preview'}));await waitFor(()=>expect(handlers.onPlay).toHaveBeenCalledTimes(2));
});

test('length changes invalidate preview without resetting seed or feedback and flow through save',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?base:request.action==='save'?{...base,items:[]}:{...base,preview:{track_ids:[1,2],analyzed:2,available:3,generation_version:'test',configuration:request.generation}});
 render(<SparkCreate {...props()}/>);await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 const slider=screen.getByRole('slider',{name:'Sparklist length'});expect(slider).toHaveValue('25');
 fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/Seed song/}));fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));await screen.findByRole('button',{name:'Listen to preview'});
 fireEvent.click(screen.getByRole('button',{name:'Less like this Second song'}));await waitFor(()=>expect(bridge.call.mock.calls.at(-1)?.[1].request.generation.less_like).toEqual([2]));
 fireEvent.click(screen.getByRole('button',{name:'Edit selection'}));const calls=bridge.call.mock.calls.length;fireEvent.change(slider,{target:{value:'100'}});expect(bridge.call).toHaveBeenCalledTimes(calls);expect(screen.queryByRole('button',{name:'Listen to preview'})).not.toBeInTheDocument();expect(screen.getByText('Starting from')).toBeVisible();
 fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));await screen.findByRole('button',{name:'Listen to preview'});expect(bridge.call.mock.calls.at(-1)?.[1].request.generation).toMatchObject({target_count:100,seed:1,less_like:[2]});expect(screen.getByText(/2 of 100 requested/)).toBeVisible();
 fireEvent.click(screen.getByRole('button',{name:'Name & save'}));fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:'Long mix'}});fireEvent.click(screen.getByRole('button',{name:'Save Sparklist'}));await waitFor(()=>expect(bridge.call.mock.calls.at(-1)?.[1].request).toMatchObject({action:'save',generation:{target_count:100}}));
});

test('combines starting songs and moods, keeps refinement drafts, and saves the exact reordered preview',async()=>{
 bridge.call.mockImplementation(async(_command,{request})=>request.action==='status'?{...base,analyzed:3,analyzed_ids:[1,2,3]}:request.action==='save'?{...base,items:[]}:{...base,preview:{track_ids:[1,2,3],configuration:request.generation,analyzed:3,available:3,generation_version:'test'}});
 render(<SparkCreate {...props()}/>);await waitFor(()=>expect(screen.getByRole('button',{name:'Choose for me'})).toBeEnabled());
 fireEvent.click(screen.getByRole('button',{name:'Select song'}));fireEvent.click(screen.getByRole('button',{name:/Seed song/}));
 fireEvent.click(screen.getByRole('button',{name:'Add starting song'}));fireEvent.click(screen.getByRole('button',{name:/Second song/}));
 fireEvent.click(screen.getByRole('button',{name:'Dreamy'}));fireEvent.click(screen.getByRole('button',{name:'Focused'}));
 fireEvent.change(screen.getByRole('slider',{name:/Variety/}),{target:{value:'75'}});
 fireEvent.change(screen.getByRole('slider',{name:/Familiar/}),{target:{value:'80'}});
 fireEvent.click(screen.getByRole('button',{name:'Preview Sparklist'}));await screen.findByRole('button',{name:'Listen to preview'});
 expect(bridge.call.mock.calls.at(-1)?.[1].request.generation.refinement).toMatchObject({seed_ids:[1,2],moods:['Dreamy','Focused'],variety:75,discovery:80});
 fireEvent.click(screen.getByRole('button',{name:'Move up Second song'}));fireEvent.click(screen.getAllByRole('button',{name:'Keep'})[0]);
 fireEvent.click(screen.getByRole('button',{name:'Name & save'}));fireEvent.change(screen.getByRole('textbox',{name:'Name'}),{target:{value:'Reviewed mix'}});fireEvent.click(screen.getByRole('button',{name:'Save Sparklist'}));
 await waitFor(()=>expect(bridge.call.mock.calls.at(-1)?.[1].request).toMatchObject({action:'save',preview:{track_ids:[2,1,3]},generation:{refinement:{kept:[{track_id:2,position:0}]}}}));
});
