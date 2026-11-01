import {afterEach,expect,test,vi} from 'vitest';
import {shuffleMorph,shuffleShape} from '../src/shuffleMorph';
import on from '../../assets/icons/ui/ShuffleON.svg?raw';
import off from '../../assets/icons/ui/ShuffleOFF.svg?raw';
afterEach(()=>vi.restoreAllMocks());
test('authored Shuffle endpoints match Sketch and reversal preserves every displayed coordinate',()=>{
 let now=0,id=0;const frames=new Map<number,FrameRequestCallback>();vi.spyOn(performance,'now').mockImplementation(()=>now);vi.spyOn(window,'requestAnimationFrame').mockImplementation(cb=>{frames.set(++id,cb);return id;});vi.spyOn(window,'cancelAnimationFrame').mockImplementation(id=>{frames.delete(id);});
 const step=(at:number)=>{now=at;const jobs=[...frames.values()];frames.clear();jobs.forEach(f=>f(at));};
 const host=document.createElement('i');host.innerHTML=off;const normalizedOff=host.innerHTML;
 const cancel=shuffleMorph(host,true,on);step(70);const shape=host.querySelectorAll('path')[1].getAttribute('d'),weight=host.querySelector('path')!.getAttribute('stroke-width');expect(weight).not.toBe('1');expect(weight).not.toBe('2');expect(shape).not.toMatch(/NaN|Infinity/);cancel();expect(frames.size).toBe(0);
 shuffleMorph(host,false,off);expect(host.querySelectorAll('path')[1].getAttribute('d')).toBe(shape);expect(host.querySelector('path')!.getAttribute('stroke-width')).toBe(weight);step(250);expect(host.innerHTML).toBe(normalizedOff);expect(frames.size).toBe(0);
 shuffleMorph(host,true,on);step(430);const expected=document.createElement('i');expected.innerHTML=on;expect(host.innerHTML).toBe(expected.innerHTML);expect(frames.size).toBe(0);
 const doc=new DOMParser().parseFromString(on,'image/svg+xml');const nums=(s:string)=>s.match(/-?\d+(?:\.\d+)?/g)!.map(Number);expect(nums(shuffleShape(1))).toEqual(nums(doc.querySelectorAll('g>path')[1].getAttribute('d')!));const values=nums(shuffleShape(0));for(let i=0;i<values.length;i+=2)expect(values[i+1]).toBeCloseTo(8+(values[i]-4)*15/24,7);
});
