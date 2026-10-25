import {act,cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({native:true,call:bridge.call,artwork:(path:string)=>path}));
vi.mock('../src/depthArtwork',()=>({useDepthArtwork:vi.fn()}));
import ArtworkViewer,{openArtwork} from '../src/ArtworkViewer';
const show=vi.fn(function(this:HTMLDialogElement){this.open=true;});
const close=vi.fn(function(this:HTMLDialogElement){this.open=false;this.dispatchEvent(new Event('close'));});
function opener(name:string){
 const button=document.createElement('button'),image=document.createElement('img');button.setAttribute('aria-label',name);image.src=`/display/${name}.jpg`;
 Object.defineProperty(image,'currentSrc',{value:`/display/${name}.jpg`});image.dataset.fullSrc=`/original/${name}.png`;image.dataset.originalArtworkPath=`/original/${name}.png`;image.dataset.artworkVersion='7';button.append(image);document.body.append(button);return button;
}
beforeEach(()=>{show.mockClear();close.mockClear();HTMLDialogElement.prototype.showModal=show;HTMLDialogElement.prototype.close=close;HTMLImageElement.prototype.decode=vi.fn().mockResolvedValue(undefined);});
afterEach(()=>{cleanup();document.querySelectorAll('body>button').forEach(el=>el.remove());bridge.call.mockReset();});
test('expansion starts with the visible SDR copy and decodes its larger copy without reopening',async()=>{
 let finish!:(path:string)=>void;bridge.call.mockImplementation(()=>new Promise<string>(resolve=>finish=resolve));render(<ArtworkViewer/>);const button=opener('wide');
 act(()=>openArtwork(button,'Wide photo'));const image=screen.getByRole('img',{name:'Wide photo'});expect(image).toHaveAttribute('src','/display/wide.jpg');expect(bridge.call).toHaveBeenCalledWith('artwork_rendition',{path:'/original/wide.png',pixels:2048});
 await act(async()=>finish('/display/wide-2048.jpg'));await waitFor(()=>expect(image).toHaveAttribute('src','/display/wide-2048.jpg'));
 expect(document.querySelector('.viewer-backdrop')).toHaveAttribute('src','/display/wide-2048.jpg');expect(show).toHaveBeenCalledTimes(1);expect(close).not.toHaveBeenCalled();
 expect(document.querySelector('img[src="/original/wide.png"]')).toBeNull();fireEvent.click(screen.getByRole('button',{name:'Close artwork'}));expect(button).toHaveFocus();
});
test('late conversion cannot replace another photo or reopen a closed viewer',async()=>{
 const pending=new Map<string,(path:string)=>void>();bridge.call.mockImplementation((_:string,args:{path:string})=>new Promise<string>(resolve=>pending.set(args.path,resolve)));render(<ArtworkViewer/>);const first=opener('first'),second=opener('second');
 act(()=>openArtwork(first,'First'));act(()=>openArtwork(second,'Second'));await act(async()=>pending.get('/original/first.png')!('/display/first-2048.jpg'));
 expect(screen.getByRole('img',{name:'Second'})).toHaveAttribute('src','/display/second.jpg');
 fireEvent.click(screen.getByRole('button',{name:'Close artwork'}));const opened=show.mock.calls.length;await act(async()=>pending.get('/original/second.png')!('/display/second-2048.jpg'));
 expect(show).toHaveBeenCalledTimes(opened);expect(document.querySelector('dialog')).not.toHaveAttribute('open');expect(second).toHaveFocus();
});
test('conversion failure keeps the already displayed photo and repeated opens reuse the cached copy',async()=>{
 bridge.call.mockRejectedValueOnce(new Error('unavailable'));render(<ArtworkViewer/>);const failed=opener('unavailable');act(()=>openArtwork(failed,'Unavailable'));await act(async()=>{});expect(screen.getByRole('img',{name:'Unavailable'})).toHaveAttribute('src','/display/unavailable.jpg');
 fireEvent.click(screen.getByRole('button',{name:'Close artwork'}));bridge.call.mockResolvedValue('/display/cached-2048.jpg');const cached=opener('cached');act(()=>openArtwork(cached,'Cached'));await waitFor(()=>expect(screen.getByRole('img',{name:'Cached'})).toHaveAttribute('src','/display/cached-2048.jpg'));
 fireEvent.click(screen.getByRole('button',{name:'Close artwork'}));act(()=>openArtwork(cached,'Cached'));await waitFor(()=>expect(screen.getByRole('img',{name:'Cached'})).toHaveAttribute('src','/display/cached-2048.jpg'));expect(bridge.call).toHaveBeenCalledTimes(2);
});
