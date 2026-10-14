import {afterEach,expect,test,vi} from 'vitest';
import {playPauseMorph} from '../src/playPauseMorph';
afterEach(()=>vi.restoreAllMocks());
test('rounded transport contours reverse from the visible state and finish on the supplied glyph',()=>{
 let now=0,next=0;const frames=new Map<number,FrameRequestCallback>();
 vi.spyOn(performance,'now').mockImplementation(()=>now);
 vi.spyOn(window,'requestAnimationFrame').mockImplementation(callback=>{frames.set(++next,callback);return next;});
 vi.spyOn(window,'cancelAnimationFrame').mockImplementation(id=>{frames.delete(id);});
 const step=(time:number)=>{now=time;const pending=[...frames.values()];frames.clear();pending.forEach(callback=>callback(time));};
 const host=document.createElement('i');document.body.append(host);
 const cancel=playPauseMorph(host,true,'<svg data-final="pause"/>');
 step(180);expect(Number(host.dataset.transportMorphProgress)).toBeCloseTo(.5);
 const contour=host.innerHTML;
 expect(host.querySelectorAll('path')).toHaveLength(4);
 for(const path of host.querySelectorAll('path')){expect(path.getAttribute('d')).toContain('Q');expect(path.getAttribute('d')).not.toMatch(/NaN|Infinity/);if(path.hasAttribute('stroke'))expect(path.getAttribute('stroke-linejoin')).toBe('round');}
 cancel();expect(frames.size).toBe(0);
 playPauseMorph(host,false,'<svg data-final="play"/>');expect(host.innerHTML).toBe(contour);
 step(540);expect(host.dataset.transportMorphProgress).toBe('0');expect(host.querySelector('[data-final=play]')).not.toBeNull();expect(host.dataset.morphing).toBeUndefined();expect(frames.size).toBe(0);
 host.remove();
});
