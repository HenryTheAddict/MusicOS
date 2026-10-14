import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,expect,test,vi} from 'vitest';
import VolumeControl from '../src/VolumeControl';
afterEach(cleanup);
test('mute restores the previous listening level and keyboard changes are labeled',async()=>{
 const change=vi.fn().mockResolvedValue(undefined);
 const {rerender}=render(<VolumeControl value={.37} onChange={change}/>);
 fireEvent.click(screen.getByRole('button',{name:'Mute'}));await waitFor(()=>expect(change).toHaveBeenLastCalledWith(0));
 rerender(<VolumeControl value={0} onChange={change}/>);
 fireEvent.click(screen.getByRole('button',{name:'Unmute'}));await waitFor(()=>expect(change).toHaveBeenLastCalledWith(.37));
 fireEvent.change(screen.getByRole('slider'),{target:{value:'.61'}});await waitFor(()=>expect(change).toHaveBeenLastCalledWith(.61));
});
test('rapid volume changes coalesce and older playback events cannot move the thumb',async()=>{
 let finish!:()=>void;
 const change=vi.fn().mockImplementationOnce(()=>new Promise<void>(resolve=>{finish=resolve;})).mockResolvedValue(undefined);
 const {rerender}=render(<VolumeControl value={.8} onChange={change}/>);
 const slider=screen.getByRole('slider');
 fireEvent.change(slider,{target:{value:'.2'}});fireEvent.change(slider,{target:{value:'.4'}});fireEvent.change(slider,{target:{value:'.7'}});
 rerender(<VolumeControl value={.2} onChange={change}/>);
 expect(slider).toHaveValue('0.7');expect(change).toHaveBeenCalledTimes(1);
 finish();await waitFor(()=>expect(change).toHaveBeenCalledTimes(2));expect(change).toHaveBeenLastCalledWith(.7);
});
test('failed volume writes restore the authoritative value',async()=>{
 render(<VolumeControl value={.4} onChange={()=>Promise.reject(new Error('offline'))}/>);
 fireEvent.change(screen.getByRole('slider'),{target:{value:'.9'}});
 await waitFor(()=>expect(screen.getByRole('alert')).toHaveTextContent('Could not change volume'));
 expect(screen.getByRole('slider')).toHaveValue('0.4');
});
