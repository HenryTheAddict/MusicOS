import {afterEach,expect,test,vi} from 'vitest';
import {playLevelsMorph} from '../src/playLevelsMorph';
afterEach(()=>vi.restoreAllMocks());
test('interrupted level morph reverses from its current contour and cancels earlier frames',()=>{
 let now=0,next=0;const frames=new Map<number,FrameRequestCallback>();
 vi.spyOn(performance,'now').mockImplementation(()=>now);
 vi.spyOn(window,'requestAnimationFrame').mockImplementation(callback=>{frames.set(++next,callback);return next;});
 vi.spyOn(window,'cancelAnimationFrame').mockImplementation(id=>{frames.delete(id);});
 const step=(time:number)=>{now=time;const pending=[...frames.values()];frames.clear();pending.forEach(callback=>callback(time));};
 const host=document.createElement('i');host.style.setProperty('--level-1','.1');host.style.setProperty('--level-2','.5');host.style.setProperty('--level-3','.9');document.body.append(host);
 const cancel=playLevelsMorph(host,true,'<svg data-final="levels"/>');expect(host.querySelector('path')?.getAttribute('d')).toMatch(/^M/);step(90);expect(Number(host.dataset.morphProgress)).toBeCloseTo(.5);const contour=host.innerHTML;cancel();expect(frames.size).toBe(0);
 playLevelsMorph(host,false,'<svg data-final="play"/>');step(90);expect(host.innerHTML).toBe(contour);step(270);expect(host.dataset.morphProgress).toBe('0');expect(host.querySelector('[data-final=play]')).not.toBeNull();expect(frames.size).toBe(0);host.remove();
});
