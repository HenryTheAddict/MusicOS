import {afterEach,expect,test,vi} from 'vitest';
import {playPauseMorph} from '../src/playPauseMorph';
afterEach(()=>vi.restoreAllMocks());
test('shape morph stays continuous, reverses from the visible state, and leaves no animation work',()=>{
 let now=0,next=0;const frames=new Map<number,FrameRequestCallback>();
 vi.spyOn(performance,'now').mockImplementation(()=>now);
 vi.spyOn(window,'requestAnimationFrame').mockImplementation(callback=>{frames.set(++next,callback);return next;});
 vi.spyOn(window,'cancelAnimationFrame').mockImplementation(id=>{frames.delete(id);});
 const step=(time:number)=>{now=time;const pending=[...frames.values()];frames.clear();pending.forEach(callback=>callback(time));};
 const play='<svg viewBox="0 0 32 32"><path data-glyph="play" d="M4 5 L27 16 L4 27 Z" fill="var(--icon-fill)" stroke="var(--icon-stroke)"/></svg>';
 const pause='<svg viewBox="0 0 32 32"><path data-glyph="pause" d="M6 4 H14 V28 H6 Z M18 4 H26 V28 H18 Z" fill="var(--icon-fill)" stroke="var(--icon-stroke)"/></svg>';
 const host=document.createElement('i');host.innerHTML=play;document.body.append(host);const canonical=host.innerHTML;
 const cancel=playPauseMorph(host,true,pause);step(120);
 const progress=host.dataset.transportMorphProgress;
 expect(Number(progress)).toBeGreaterThan(0);expect(Number(progress)).toBeLessThan(1);
 expect(host.querySelector('[data-transport-morph]')).not.toBeNull();
 expect(host.querySelectorAll('path')).toHaveLength(5);
 const clips=[...host.querySelectorAll('path')].map(p=>p.getAttribute('d'));
 clips.forEach(d=>expect(d).not.toMatch(/NaN|Infinity/));
 cancel();expect(frames.size).toBe(0);
 playPauseMorph(host,false,play);expect(host.dataset.transportMorphProgress).toBe(progress);
 expect([...host.querySelectorAll('path')].map(p=>p.getAttribute('d'))).toEqual(clips);
 step(480);expect(host.innerHTML).toBe(canonical);expect(host.dataset.transportMorphProgress).toBe('0');expect(host.dataset.morphing).toBeUndefined();expect(frames.size).toBe(0);
 host.remove();
});

test('sampled transport contours retain supplied widths and never stretch beyond their endpoints',async()=>{
 const {transportContours}=await import('../src/playPauseMorph');
 const bounds=(progress:number)=>{const xs=transportContours(progress).flat().map(p=>p[0]);return {left:Math.min(...xs),right:Math.max(...xs),width:Math.max(...xs)-Math.min(...xs)};};
 expect(bounds(0).width).toBeCloseTo(22.764,1);expect(bounds(1).width).toBeCloseTo(20,1);
 let previous=bounds(0);
 for(let i=1;i<=100;i++){const current=bounds(i/100);expect(current.left).toBeGreaterThanOrEqual(4-.01);expect(current.right).toBeLessThanOrEqual(26.764+.01);expect(Math.abs(current.width-previous.width)).toBeLessThan(.15);previous=current;}
});

test('each transport half owns its bar without overlapping triangles or a width surge',async()=>{
 const {transportContours}=await import('../src/playPauseMorph');
 for(let step=0;step<=100;step++){
  const [left,right]=transportContours(step/100);
  expect(Math.max(...left.map(p=>p[0]))).toBeLessThanOrEqual(Math.min(...right.map(p=>p[0]))+.001);
  const xs=[...left,...right].map(p=>p[0]);
  expect(Math.max(...xs)-Math.min(...xs)).toBeGreaterThan(19.9);
  expect(Math.max(...xs)-Math.min(...xs)).toBeLessThan(22.8);
 }
});

test('joined triangle has no interior stroke; pause reveals both inner edges',async()=>{
 const {transportDrawing}=await import('../src/playPauseMorph');
 const joined=transportDrawing(0),separate=transportDrawing(1);
 expect(joined.fill.match(/ Z/g)).toHaveLength(2);
 expect(joined.seams.every(d=>d.includes('L'))).toBe(true);
 expect(separate.outlines.every(d=>d.includes('L'))).toBe(true);
});
