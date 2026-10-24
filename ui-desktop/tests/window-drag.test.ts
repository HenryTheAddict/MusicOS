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

it('long focusable song labels remain draggable without stealing keyboard or control clicks',()=>{
 document.body.innerHTML='<section class="mini-player"><strong><span class="overflow-label" tabindex="0"><span>Very long song title</span></span></strong><button><span class="overflow-label" tabindex="0">Album link</span></button><div tabindex="0">Interactive surface</div></section>';
 dragWindow(event(document.querySelector('.overflow-label span')!),vi.fn());
 dragWindow(event(document.querySelector('button span')!),vi.fn());dragWindow(event(document.querySelector('div')!),vi.fn());
 expect(mocks.start).toHaveBeenCalledOnce();
});
it('never races a menu expansion or collapse with a native drag',()=>{
 document.body.innerHTML='<section class="mini-player" style="--mini-player-height:120px"><strong>Song title</strong></section>';
 dragWindow(event(document.querySelector('strong')!),vi.fn());expect(mocks.start).not.toHaveBeenCalled();
 document.querySelector<HTMLElement>('section')!.style.removeProperty('--mini-player-height');dragWindow(event(document.querySelector('strong')!),vi.fn());expect(mocks.start).toHaveBeenCalledOnce();
});
it('preserves editable text and draggable queue items',()=>{
 document.body.innerHTML='<div contenteditable="true"><span>Draft</span></div><div draggable="true"><strong>Queue item</strong></div>';
 for(const target of document.querySelectorAll('span,strong'))dragWindow(event(target),vi.fn());expect(mocks.start).not.toHaveBeenCalled();
});
