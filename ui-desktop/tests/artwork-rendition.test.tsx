import {act,cleanup,render,screen,waitFor} from '@testing-library/react';
import {afterEach,beforeEach,expect,test,vi} from 'vitest';
const bridge=vi.hoisted(()=>({call:vi.fn()}));
vi.mock('../src/bridge',()=>({native:true,call:bridge.call,artwork:(path:string)=>path,asset:()=>'/placeholder.svg',libraryArtwork:vi.fn(),localArtwork:vi.fn()}));
import {Cover} from '../src/components';
const intersects:Function[]=[],resizes:Function[]=[];
beforeEach(()=>{
 HTMLImageElement.prototype.decode=vi.fn().mockResolvedValue(undefined);intersects.length=0;resizes.length=0;
 vi.stubGlobal('IntersectionObserver',class{constructor(callback:Function){intersects.push(callback);}observe(){}disconnect(){}});
 vi.stubGlobal('ResizeObserver',class{constructor(callback:Function){resizes.push(callback);}observe(){}disconnect(){}});
});
afterEach(()=>{cleanup();vi.unstubAllGlobals();bridge.call.mockReset();});
test('offscreen artwork never loads the original; visible views request bounded cache sizes',async()=>{
 let complete!:(path:string)=>void;bridge.call.mockImplementation(()=>new Promise<string>(resolve=>complete=resolve));
 render(<Cover src="/huge-photo.png" title="Huge photo"/>);
 const image=screen.getByRole('img');expect(image).toHaveAttribute('src','/placeholder.svg');expect(image).not.toHaveAttribute('data-artwork-path');expect(bridge.call).not.toHaveBeenCalled();
 Object.defineProperty(image,'clientWidth',{value:700});
 act(()=>intersects.forEach(callback=>callback([{isIntersecting:true}])));
 expect(bridge.call).toHaveBeenCalledWith('artwork_rendition',{path:'/huge-photo.png',pixels:1200});
 expect(image).toHaveAttribute('src','/placeholder.svg');
 await act(async()=>complete('/cache/1200.jpg'));
 await waitFor(()=>expect(image).toHaveAttribute('src','/cache/1200.jpg'));
 expect(image).toHaveAttribute('data-artwork-path','/huge-photo.png');expect(image).toHaveAttribute('data-full-src','/huge-photo.png');
});
test('late small-rendition completion cannot overwrite a larger resize result',async()=>{
 const complete:((path:string)=>void)[]=[];bridge.call.mockImplementation(()=>new Promise<string>(resolve=>complete.push(resolve)));
 render(<Cover src="/resize-photo.png" title="Resize photo"/>);const image=screen.getByRole('img');let width=60;
 Object.defineProperty(image,'clientWidth',{get:()=>width});
 act(()=>intersects.forEach(callback=>callback([{isIntersecting:true}])));
 width=800;act(()=>resizes.forEach(callback=>callback()));
 await act(async()=>complete[1]('/cache/large.jpg'));await waitFor(()=>expect(image).toHaveAttribute('src','/cache/large.jpg'));
 await act(async()=>complete[0]('/cache/tiny.jpg'));expect(image).toHaveAttribute('src','/cache/large.jpg');
});
test('multiple views deduplicate the same original revision and requested size',async()=>{
 bridge.call.mockResolvedValue('/cache/shared.jpg');render(<><Cover src="/shared.png" title="First"/><Cover src="/shared.png" title="Second"/></>);
 act(()=>intersects.forEach(callback=>callback([{isIntersecting:true}])));
 await waitFor(()=>expect(screen.getAllByRole('img').every(image=>image.getAttribute('src')==='/cache/shared.jpg')).toBe(true));expect(bridge.call).toHaveBeenCalledTimes(1);
});
