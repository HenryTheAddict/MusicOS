import {expect,test,vi} from 'vitest';
import {jelly,parallaxOffset,springStep} from '../src/motion';
test('spring is independent of refresh rate and keeps reversals continuous',()=>{
 const simulate=(hz:number)=>{let x=0,v=0;for(let i=0;i<hz*.4;i++)[x,v]=springStep(x,v,i/hz<.085?.009:0,1/hz);return x;};
 expect(Math.abs(simulate(60)-simulate(120))).toBeLessThan(.001);
 let x=.01,v=-.08;const before=x;[x,v]=springStep(x,v,0,1/120);expect(Math.abs(x-before)).toBeLessThan(.002);expect(Number.isFinite(v)).toBe(true);
});
test('rapid hover reversal reuses the live spring and never writes transform',()=>{
 const raf=vi.spyOn(window,'requestAnimationFrame').mockReturnValue(1);const element=document.createElement('div');document.body.append(element);
 const first=jelly(element,true);expect(jelly(element,false,first)).toBe(first);expect(element.style.transform).toBe('');first.cancel();element.remove();raf.mockRestore();
});
test('scroll displacement is bounded to fourteen pixels',()=>{expect(parallaxOffset(0,0,800)).toBe(-14);expect(parallaxOffset(400,0,800)).toBe(0);expect(parallaxOffset(800,0,800)).toBe(14);expect(parallaxOffset(2000,0,800)).toBe(14);});
