import {act,cleanup,renderHook} from '@testing-library/react';
import {afterEach,expect,it,vi} from 'vitest';
import {useLyricFollow} from '../src/useLyricFollow';
import type {LyricDocument} from '../src/lyrics';
afterEach(()=>{cleanup();vi.unstubAllGlobals();document.body.replaceChildren();});
it('moves continuously between mounted cues, reverses from the displayed offset and stops after completion',()=>{
 const host=document.createElement('div');document.body.append(host);
 Object.defineProperties(host,{clientHeight:{value:200},scrollHeight:{value:1200}});
 host.getBoundingClientRect=()=>({top:0} as DOMRect);
 for(let i=0;i<3;i++){const line=document.createElement('button');line.dataset.line=String(i);line.getBoundingClientRect=()=>({top:100+i*150-host.scrollTop,bottom:140+i*150-host.scrollTop} as DOMRect);host.append(line);}
 let serial=0;const frames=new Map<number,FrameRequestCallback>();
 vi.stubGlobal('requestAnimationFrame',(cb:FrameRequestCallback)=>{frames.set(++serial,cb);return serial;});vi.stubGlobal('cancelAnimationFrame',(id:number)=>frames.delete(id));
 const ref={current:host},doc={} as LyricDocument;
 const {rerender,unmount}=renderHook(({focus,following})=>useLyricFollow(ref,{active:true,following,doc,trackId:1,variant:'visualizer',focus,last:focus,layoutEpoch:0}),{initialProps:{focus:0,following:true}});
 expect(frames.size).toBe(0);const initial=host.scrollTop;
 const run=(at:number)=>{const jobs=[...frames.values()];frames.clear();act(()=>jobs.forEach(job=>job(at)));};
 rerender({focus:1,following:true});const began=performance.now();run(began+100);expect(host.scrollTop).toBeGreaterThan(initial);const midway=host.scrollTop;
 rerender({focus:0,following:true});expect(host.scrollTop).toBe(midway);run(performance.now()+240);expect(host.scrollTop).toBeCloseTo(initial);expect(frames.size).toBe(0);
 rerender({focus:2,following:true});expect(frames.size).toBe(1);rerender({focus:2,following:false});expect(frames.size).toBe(0);unmount();expect(frames.size).toBe(0);
});
