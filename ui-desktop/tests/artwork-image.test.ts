import {act,renderHook,waitFor,cleanup} from '@testing-library/react';
import {afterEach,expect,it,vi} from 'vitest';
import {useArtworkImage,type ArtworkImage} from '../src/useArtworkImage';
afterEach(()=>{cleanup();vi.unstubAllGlobals();});
function deferredImages(){const loads:{resolve:()=>void;reject:()=>void}[]=[];vi.stubGlobal('Image',class{src='';decode(){return new Promise<void>((resolve,reject)=>loads.push({resolve,reject}));}});return loads;}
const image=(name:string):ArtworkImage=>({url:`/${name}.jpg`,path:`/${name}`,original:`/${name}`,version:1});
it('keeps the old photo and depth identity visible until its replacement decodes',async()=>{
 const loads=deferredImages(),failed=vi.fn();const {result,rerender}=renderHook(({wanted})=>useArtworkImage(wanted,'album',true,failed),{initialProps:{wanted:image('old')}});rerender({wanted:image('new')});expect(result.current).toEqual(image('old'));await act(async()=>loads[0].resolve());await waitFor(()=>expect(result.current).toEqual(image('new')));
});
it('ignores an obsolete completion and keeps valid artwork during a failed replacement',async()=>{
 const loads=deferredImages(),failed=vi.fn();const {result,rerender}=renderHook(({wanted})=>useArtworkImage(wanted,'artist',true,failed),{initialProps:{wanted:image('old')}});rerender({wanted:image('obsolete')});rerender({wanted:image('latest')});await act(async()=>loads[0].resolve());expect(result.current).toEqual(image('old'));await act(async()=>loads[1].reject());await waitFor(()=>expect(failed).toHaveBeenCalledOnce());expect(result.current).toEqual(image('old'));
});
it('never carries an image into a different album or an explicit artwork clear',()=>{
 deferredImages();const {result,rerender}=renderHook(({wanted,id,keep})=>useArtworkImage(wanted,id,keep,vi.fn()),{initialProps:{wanted:image('one'),id:'one',keep:true}});rerender({wanted:image('two'),id:'two',keep:true});expect(result.current).toEqual(image('two'));rerender({wanted:image('placeholder'),id:'two',keep:false});expect(result.current).toEqual(image('placeholder'));
});
it('retains an existing decoded frame while its new cached rendition is preparing',async()=>{
 const loads=deferredImages();const {result,rerender}=renderHook(({wanted,pending})=>useArtworkImage(wanted,'album',true,vi.fn(),pending),{initialProps:{wanted:image('old'),pending:false}});
 rerender({wanted:image('placeholder'),pending:true});expect(result.current).toEqual(image('old'));expect(loads).toHaveLength(0);
 rerender({wanted:image('cached-new'),pending:false});await act(async()=>loads[0].resolve());expect(result.current).toEqual(image('cached-new'));
});
