import {afterEach,expect,test,vi} from 'vitest';
import {menuCrossMorph} from '../src/menuCrossMorph';
afterEach(()=>vi.restoreAllMocks());
test('creation cross keeps its strokes and center and reverses from the visible angle',()=>{
 let now=0,next=0;const frames=new Map<number,FrameRequestCallback>();
 vi.spyOn(performance,'now').mockImplementation(()=>now);
 vi.spyOn(window,'requestAnimationFrame').mockImplementation(callback=>{frames.set(++next,callback);return next;});
 vi.spyOn(window,'cancelAnimationFrame').mockImplementation(id=>{frames.delete(id);});
 const step=(time:number)=>{now=time;const pending=[...frames.values()];frames.clear();pending.forEach(callback=>callback(time));};
 const host=document.createElement('i'),final='<svg data-end="closed"></svg>';
 const cancel=menuCrossMorph(host,true,final);step(55);
 const angle=host.querySelector('path')!.getAttribute('transform'),progress=host.dataset.crossProgress;
 expect(host.querySelectorAll('path')).toHaveLength(1);expect(host.querySelector('path')).toHaveAttribute('d','M7 16H25M16 7V25');
 expect(host.querySelector('path')).toHaveAttribute('stroke-width','2');
 expect(Number(progress)).toBeGreaterThan(0);expect(Number(progress)).toBeLessThan(1);
 cancel();menuCrossMorph(host,false,final);expect(host.querySelector('path')).toHaveAttribute('transform',angle!);expect(host.dataset.crossProgress).toBe(progress);
 step(200);expect(host.dataset.crossProgress).toBe('0');expect(host.innerHTML).toBe(final);expect(frames.size).toBe(0);
});
