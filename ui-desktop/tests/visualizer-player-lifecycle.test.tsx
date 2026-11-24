import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {cleanup,fireEvent,render,waitFor,within} from '@testing-library/react';
import type {ComponentProps} from 'react';
import VisualizerPlayer from '../src/VisualizerPlayer';
import {defaultVisualizer} from '../src/visualizer';
const state=vi.hoisted(()=>({native:false,full:false,browserFull:null as Element|null,beatSync:undefined as boolean|undefined,call:vi.fn(),setFull:vi.fn()}));
vi.mock('../src/bridge',()=>({get native(){return state.native;},call:state.call,errorText:String}));
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async()=>()=>{})}));
vi.mock('@tauri-apps/api/window',()=>({getCurrentWindow:()=>({isFullscreen:async()=>state.full,setFullscreen:state.setFull})}));
vi.mock('../src/visualizerAlbumPalette',()=>({useVisualizerAlbumPalette:()=>({pending:false,palette:undefined})}));
vi.mock('../src/VisualizerPicker',()=>({default:()=>null}));
vi.mock('../src/VisualizerSettings',()=>({default:()=>null}));
vi.mock('../src/BeatPulseGrid',()=>({default:({beatSync}:{beatSync:boolean})=>{state.beatSync=beatSync;return null;}}));
vi.mock('../src/ExpandedPlayer',()=>({default:({visualizerStatus}:ComponentProps<typeof import('../src/ExpandedPlayer').default>)=>{
 return <><button onClick={()=>{if(state.native)void state.setFull(!state.full);else if(document.fullscreenElement)void document.exitFullscreen();else void document.documentElement.requestFullscreen();}}>Fullscreen</button>{visualizerStatus}</>;
}}));
function props(preset='beat-pulse'){
 return {preferences:{visualizer:{...defaultVisualizer,preset}},onPreferences:vi.fn(),onClose:vi.fn(),onView:vi.fn(),player:{is_playing:true},theme:{},open:true,view:'playing'} as unknown as ComponentProps<typeof VisualizerPlayer>;
}
beforeEach(()=>{
 state.native=false;state.full=false;state.browserFull=null;state.call.mockReset();state.setFull.mockReset();
 state.call.mockImplementation(async(_command,args)=>({requestId:args.request.settings?.requestId,phase:'running',preset:args.request.settings?.preferences.preset}));
 state.setFull.mockImplementation(async(full:boolean)=>{state.full=full;});
 Object.defineProperty(document,'fullscreenElement',{configurable:true,get:()=>state.browserFull});
 document.documentElement.requestFullscreen=vi.fn(async()=>{state.browserFull=document.documentElement;});
 document.exitFullscreen=vi.fn(async()=>{state.browserFull=null;});
 delete document.documentElement.dataset.nativeVisualizer;
});
afterEach(async()=>{cleanup();await new Promise(resolve=>setTimeout(resolve,0));vi.restoreAllMocks();});
it.each([false,true])('uses browser fullscreen without closing and restores on unmount (initial fullscreen %s)',async initiallyFull=>{
 if(initiallyFull)state.browserFull=document.documentElement;
 const p=props(),view=render(<VisualizerPlayer {...p}/>),scope=within(view.container);
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));expect(!!state.browserFull).toBe(!initiallyFull);expect(p.onClose).not.toHaveBeenCalled();
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));expect(!!state.browserFull).toBe(initiallyFull);
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));view.unmount();
 await waitFor(()=>expect(!!state.browserFull).toBe(initiallyFull));
});
it.each([false,true])('keeps preset changes windowed and restores desktop fullscreen on close (initial fullscreen %s)',async initiallyFull=>{
 state.native=true;state.full=initiallyFull;
 const p=props(),view=render(<VisualizerPlayer {...p}/>),scope=within(view.container);
 await waitFor(()=>expect(scope.queryByRole('status')).toBeNull());
 expect(state.full).toBe(true);
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));expect(state.full).toBe(false);expect(p.onClose).not.toHaveBeenCalled();
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));expect(state.full).toBe(true);
 fireEvent.click(scope.getByRole('button',{name:'Fullscreen'}));state.setFull.mockClear();
 view.rerender(<VisualizerPlayer {...p} preferences={{...p.preferences,visualizer:{...defaultVisualizer,preset:'boxes'}}}/>);
 await waitFor(()=>expect(state.call).toHaveBeenCalledWith('visualizer',expect.objectContaining({request:expect.objectContaining({action:'open'})})));
 expect(state.full).toBe(false);
 view.rerender(<VisualizerPlayer {...p}/>);
 await waitFor(()=>expect(state.call).toHaveBeenCalledWith('visualizer',{request:{action:'close'}}));
 await waitFor(()=>expect(scope.queryByRole('status')).toBeNull());
 expect(state.full).toBe(false);expect(state.setFull).not.toHaveBeenCalled();
 view.unmount();await waitFor(()=>expect(state.setFull).toHaveBeenCalledExactlyOnceWith(initiallyFull));
 expect(state.full).toBe(initiallyFull);
});
it('restores browser fullscreen when switching back to a native preset',async()=>{
 const p=props(),view=render(<VisualizerPlayer {...p}/>);fireEvent.click(within(view.container).getByRole('button',{name:'Fullscreen'}));expect(state.browserFull).not.toBeNull();
 view.rerender(<VisualizerPlayer {...p} preferences={{...p.preferences,visualizer:{...defaultVisualizer,preset:'boxes'}}}/>);
 await waitFor(()=>expect(state.browserFull).toBeNull());
});
it('passes changes to the saved Beat Sync setting into the web grid',()=>{
 const p=props(),view=render(<VisualizerPlayer {...p}/>);expect(state.beatSync).toBe(true);
 view.rerender(<VisualizerPlayer {...p} preferences={{...p.preferences,visualizer:{...defaultVisualizer,preset:'beat-pulse',beat_sync:false}}}/>);expect(state.beatSync).toBe(false);
});
it('renders moving icons without invoking the native renderer',async()=>{
 state.native=true;
 const view=render(<VisualizerPlayer {...props('moving-icons')}/>);
 await waitFor(()=>expect(within(view.container).queryByRole('status')).toBeNull());
 expect(state.call).not.toHaveBeenCalled();
 expect(view.container.querySelectorAll('.moving-icons-cell')).toHaveLength(15);
});
