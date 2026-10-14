import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({call:bridge.call,native:true,errorText:String}));
vi.mock('../src/components',()=>({Icon:()=>null}));
import AudioAnalysis from '../src/AudioAnalysis';
import type {Library} from '../src/types';
afterEach(()=>{cleanup();vi.clearAllMocks();});
test('audio rescan defaults to stale files and full recheck is explicit',async()=>{
 bridge.call.mockResolvedValue({running:false,paused:false,processed:0,total:0,features:[],errors:[]});
 render(<AudioAnalysis library={{tracks:[{id:1,available:true}]} as Library}/>);
 await waitFor(()=>expect(bridge.call).toHaveBeenCalledWith('audio_analysis',{action:'status'}));
 fireEvent.click(screen.getByRole('button',{name:'Rescan audio'}));await waitFor(()=>expect(bridge.call).toHaveBeenLastCalledWith('audio_analysis',{action:'rescan'}));
 fireEvent.click(screen.getByRole('switch',{name:'Recheck every available song'}));fireEvent.click(screen.getByRole('button',{name:'Rescan audio'}));await waitFor(()=>expect(bridge.call).toHaveBeenLastCalledWith('audio_analysis',{action:'rescan_all'}));
});
