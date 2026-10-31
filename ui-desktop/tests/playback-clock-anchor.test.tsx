import {act,renderHook} from '@testing-library/react';
import {test,expect,vi,afterEach} from 'vitest';
import {usePlaybackFeed,currentPlaybackClock,clockPosition} from '../src/playbackFeed';
afterEach(()=>vi.restoreAllMocks());
test('meter-only updates retain source receipt time while seek and speed reanchor',()=>{
 let at=1000;vi.spyOn(performance,'now').mockImplementation(()=>at);const initial={track:{id:81},position:10,is_playing:true,volume:1,queue:[],shuffle:false,repeat:'none',listening:{speed:1}} as any;
 const hook=renderHook(()=>usePlaybackFeed(initial));act(()=>hook.result.current[1](initial));expect(currentPlaybackClock().receivedAt).toBe(1000);
 at=1200;act(()=>hook.result.current[1]({...initial,audio_levels:[.8,.5,.2]}));expect(currentPlaybackClock().receivedAt).toBe(1000);expect(clockPosition(currentPlaybackClock())).toBeCloseTo(10.2);
 at=1250;act(()=>hook.result.current[1]({...initial,position:10.25}));expect(currentPlaybackClock().receivedAt).toBe(1250);
 at=1300;act(()=>hook.result.current[1]({...initial,position:30}));expect(clockPosition(currentPlaybackClock())).toBe(30);
 at=1350;act(()=>hook.result.current[1]({...initial,position:30,listening:{speed:2}}));at=1450;expect(clockPosition(currentPlaybackClock())).toBeCloseTo(30.2);hook.unmount();
});
