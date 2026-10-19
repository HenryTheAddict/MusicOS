import {afterEach,expect,it,vi} from 'vitest';
import type {MouseEvent} from 'react';
const mocks=vi.hoisted(()=>({start:vi.fn().mockResolvedValue(undefined)}));
vi.mock('../src/bridge',()=>({native:true}));
vi.mock('@tauri-apps/api/window',()=>({getCurrentWindow:()=>({startDragging:mocks.start})}));
import {dragWindow} from '../src/windowDrag';
afterEach(()=>{document.body.replaceChildren();mocks.start.mockClear();});
function event(target:Element,button=0){return {target,button,defaultPrevented:false,preventDefault:vi.fn()} as unknown as MouseEvent<HTMLElement>;}
it('starts a native drag from nested header text',()=>{
 document.body.innerHTML='<header><strong><span>Song title</span></strong></header>';
 const e=event(document.querySelector('span')!);dragWindow(e,vi.fn());expect(mocks.start).toHaveBeenCalledOnce();expect(e.preventDefault).toHaveBeenCalledOnce();
});
it('keeps controls, queue rows, and right clicks interactive',()=>{
 document.body.innerHTML='<button><span>Play</span></button><div class="queue-scroll"><strong>Next song</strong></div><div class="playback-progress"><i></i></div>';
 for(const target of document.querySelectorAll('span,strong,i'))dragWindow(event(target),vi.fn());dragWindow(event(document.body,2),vi.fn());expect(mocks.start).not.toHaveBeenCalled();
});
it('reports a failed native move instead of discarding it',async()=>{
 mocks.start.mockRejectedValueOnce(Error('window'));const failed=vi.fn();dragWindow(event(document.body),failed);await vi.waitFor(()=>expect(failed).toHaveBeenCalledWith(expect.stringContaining('Could not move')));
});
