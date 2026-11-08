import {expect,it} from 'vitest';
import {sourcePosition,sourceCorrection,type SourceClock} from '../src/playbackClock';
const clock:SourceClock={receivedAt:1000,position:10,playing:true,speed:1,trackId:1};
it('keeps the displayed source position continuous through jitter, then converges',()=>{
 const next={...clock,receivedAt:1200,position:10.16};const correction=sourceCorrection(clock,next,1200),value={...next,correction};
 expect(sourcePosition(value,1200)).toBeCloseTo(10.2);expect(sourcePosition(value,1600)).toBeCloseTo(10.56,1);
 let before=sourcePosition(value,1200);for(let at=1210;at<1650;at+=10){const after=sourcePosition(value,at);expect(after).toBeGreaterThanOrEqual(before);before=after;}
});
it('reanchors seeks, handoffs, pauses and speed changes without drifting',()=>{
 for(const next of [{...clock,position:40},{...clock,trackId:2},{...clock,playing:false},{...clock,speed:1.5}])expect(sourceCorrection(clock,next,1100)).toBe(0);
 expect(sourcePosition({...clock,playing:false,correction:.1},1500)).toBe(10);
 expect(sourcePosition(clock,9000)).toBe(10.5);
});
