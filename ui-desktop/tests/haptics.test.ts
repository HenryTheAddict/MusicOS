import {expect,test,vi} from 'vitest';
vi.mock('../src/bridge',()=>({native:true,call:vi.fn(()=>Promise.resolve())}));
vi.mock('../src/playbackFeed',()=>({currentPlaybackClock:()=>({trackId:1,playing:false,receivedAt:0,position:0,speed:1}),clockPosition:()=>0,subscribePlaybackClock:vi.fn(()=>()=>{})}));
vi.mock('../src/heartVisibility',()=>({beatOffsets:new Map()}));
import {nextBeat,setFeedbackSettings,haptic,hoverHeart} from '../src/haptics';
import {call} from '../src/bridge';
test('beat scheduler skips complete beats above four pulses per second',()=>{expect(nextBeat(.25,120,1,0)).toEqual({index:1,delay:250});const fast=nextBeat(.05,400,2,0);expect(fast.index%4).toBe(0);expect(fast.delay).toBeCloseTo(275);expect(nextBeat(4,120,1,.25).index).toBe(8);});
test('disabled, unfocused, and inactive hearts do not send feedback',()=>{vi.spyOn(document,'hasFocus').mockReturnValue(true);setFeedbackSettings({enabled:false,beat_hover:true,interactions:true});haptic('drop','drop');expect(call).not.toHaveBeenCalled();setFeedbackSettings({enabled:true,beat_hover:false,interactions:true});haptic('beat','1',1);expect(call).not.toHaveBeenCalled();haptic('drop','drop');expect(call).toHaveBeenCalledTimes(1);const node=document.createElement('button');document.body.append(node);hoverHeart(node);hoverHeart(null);expect(call).toHaveBeenCalledTimes(1);node.remove();vi.restoreAllMocks();});
