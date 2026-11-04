import {renderHook,act} from '@testing-library/react';
import {it,expect,vi,afterEach} from 'vitest';
import {useLyricGlow} from '../src/useLyricGlow';
afterEach(()=>{vi.unstubAllGlobals();document.body.replaceChildren();});
it('keeps active ink outside the mask, coalesces measurement and releases it without its own playback loop',()=>{
 const frames=new Map<number,FrameRequestCallback>();let serial=0,resize=()=>{};
 vi.stubGlobal('requestAnimationFrame',(cb:FrameRequestCallback)=>{frames.set(++serial,cb);return serial;});vi.stubGlobal('cancelAnimationFrame',(id:number)=>frames.delete(id));
 vi.stubGlobal('ResizeObserver',class{constructor(cb:()=>void){resize=cb;}observe(){}disconnect(){}});
 vi.stubGlobal('matchMedia',()=>({matches:false,addEventListener(){},removeEventListener(){}}));
 const root=document.createElement('div');root.className='app-shell';root.innerHTML='<section class="lyrics-pane"><div class="lyrics-lines"><button class="lyric-line" data-current="true"><span class="lyric-word" data-word-active="true" data-word-start="1000" data-word-end="2000">Hello</span></button></div></section>';document.body.append(root);
 const pane={current:root.querySelector<HTMLElement>('.lyrics-pane')!},scroll={current:root.querySelector<HTMLDivElement>('.lyrics-lines')!};
 const {result,rerender,unmount}=renderHook(({active})=>useLyricGlow(pane,scroll,active,'song:word','full',null),{initialProps:{active:true}});
 const layer=root.querySelector<HTMLElement>('.lyric-glow-layer')!;expect(layer.parentElement).toBe(root);expect(layer.getAttribute('aria-hidden')).toBe('true');expect(layer.querySelector('.lyric-line')).toBeNull();expect(layer.querySelector('.lyric-glow-clone')).not.toBeNull();
 act(()=>result.current.tick(1500));expect(Number(layer.querySelector<HTMLElement>('.lyric-word')!.style.getPropertyValue('--glow-strength'))).toBeGreaterThan(.4);expect(frames.size).toBe(0);
 resize();resize();expect(frames.size).toBe(1);const queued=[...frames.values()];frames.clear();act(()=>queued.forEach(cb=>cb(0)));expect(frames.size).toBe(0);
 resize();expect(frames.size).toBe(1);rerender({active:false});expect(frames.size).toBe(0);expect(root.querySelector('.lyric-glow-layer')).toBeNull();unmount();expect(frames.size).toBe(0);
});
