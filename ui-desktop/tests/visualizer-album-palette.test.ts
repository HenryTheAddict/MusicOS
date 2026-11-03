import {act,renderHook,waitFor} from '@testing-library/react';
import {expect,it,vi} from 'vitest';
import {resolveArtworkTheme} from '../src/artworkTheme';
import {useVisualizerAlbumPalette,visualizerAlbumPalette} from '../src/visualizerAlbumPalette';
vi.mock('../src/artworkTheme',()=>({resolveArtworkTheme:vi.fn()}));
const palette={accent:'#800000',blob:'#ff8080',background:'#ffe0e0',text:'#000000'};
it('uses actual artwork colors independently of theme text and rejects malformed colors',()=>{
 expect(visualizerAlbumPalette(palette)?.[0]).toEqual([128/255,0,0]);
 expect(visualizerAlbumPalette({...palette,blob:'invalid'})).toBeUndefined();
});
it('prefers distinct raw cover swatches over whitened interface surfaces',()=>{
 expect(visualizerAlbumPalette({...palette,swatches:['#1234ab','#e3a211','#be2844']})).toEqual([[18/255,52/255,171/255],[227/255,162/255,17/255],[190/255,40/255,68/255]]);
});
it('does no artwork work when off, rejects stale albums, and restores original colors immediately',async()=>{
 let finishA:(value:any)=>void=()=>{},finishB:(value:any)=>void=()=>{};
 vi.mocked(resolveArtworkTheme).mockClear().mockImplementationOnce(()=>new Promise(r=>{finishA=r;})).mockImplementationOnce(()=>new Promise(r=>{finishB=r;}));
 const {result,rerender}=renderHook(({enabled,id})=>useVisualizerAlbumPalette(enabled,id,`cover-${id}`),{initialProps:{enabled:false,id:1}});
 expect(resolveArtworkTheme).not.toHaveBeenCalled();rerender({enabled:true,id:1});rerender({enabled:true,id:2});
 await act(async()=>finishA({palette,path:'cover-1'}));expect(result.current).toBeUndefined();
 await act(async()=>finishB({palette,path:'cover-2'}));await waitFor(()=>expect(result.current).toEqual(visualizerAlbumPalette(palette)));
 rerender({enabled:false,id:2});expect(result.current).toBeUndefined();
});
