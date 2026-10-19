import {act,cleanup,fireEvent,render,screen} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import QuickAudio from '../src/QuickAudio';
import {emptyPlayback,defaultPreferences} from '../src/bridge';
vi.mock('../src/bridge',async()=>{const original=await vi.importActual<typeof import('../src/bridge')>('../src/bridge');return {...original,call:vi.fn().mockResolvedValue(null)};});
afterEach(()=>{cleanup();vi.useRealTimers();});
test('unconfirmed audio settings expose a retry and clear after mixer acknowledgement',()=>{
 vi.useFakeTimers();HTMLElement.prototype.showPopover=vi.fn();HTMLElement.prototype.hidePopover=vi.fn();
 const prefs={...defaultPreferences,listening:{bands:3 as const,gains3:[0,0,0],gains8:Array(8).fill(0),speed:1,pitch:0,linked:true,reverb:0.45,reverb_depth:.5}},onChange=vi.fn();
 const props={player:{...emptyPlayback,listening_applied:false,applied_reverb:0},preferences:prefs,onChange,onVolume:vi.fn(),error:''};const view=render(<QuickAudio {...props}/>);
 act(()=>{window.dispatchEvent(new CustomEvent('musicos-quick-audio',{detail:{anchor:document.createElement('button'),track:null}}));vi.advanceTimersByTime(2600);});expect(screen.getByRole('alert',{hidden:true})).toHaveTextContent('Playback has not confirmed');fireEvent.click(screen.getByRole('button',{name:'Retry audio controls',hidden:true}));expect(onChange).toHaveBeenCalledWith(prefs);
 view.rerender(<QuickAudio {...props} player={{...props.player,listening_applied:true,applied_reverb:0.45}}/>);expect(screen.queryByRole('alert',{hidden:true})).not.toBeInTheDocument();
});
