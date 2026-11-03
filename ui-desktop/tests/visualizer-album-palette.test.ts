import {act,cleanup,renderHook,waitFor} from '@testing-library/react';
import {afterEach,expect,it,vi} from 'vitest';
import {resolveArtworkTheme} from '../src/artworkTheme';
import {useVisualizerAlbumPalette,visualizerAlbumPalette} from '../src/visualizerAlbumPalette';
vi.mock('../src/artworkTheme',()=>({resolveArtworkTheme:vi.fn()}));
afterEach(cleanup);
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
 await act(async()=>finishA({palette,path:'cover-1'}));expect(result.current.palette).toBeUndefined();
 await act(async()=>finishB({palette,path:'cover-2'}));await waitFor(()=>expect(result.current.palette).toEqual(visualizerAlbumPalette(palette)));
 rerender({enabled:false,id:2});expect(result.current.palette).toBeUndefined();
});
it('keeps ready album colors on same-album skips and restores a previously read cover synchronously',async()=>{
 const green={...palette,swatches:['#004400','#33aa66','#bbeebb']};
 vi.mocked(resolveArtworkTheme).mockReset().mockResolvedValueOnce({palette,path:'red'}).mockResolvedValueOnce({palette:green,path:'green'});
 const {result,rerender}=renderHook(({id,cover})=>useVisualizerAlbumPalette(true,id,cover),{initialProps:{id:10,cover:'red'}});
 await waitFor(()=>expect(result.current.palette).toEqual(visualizerAlbumPalette(palette)));
 rerender({id:11,cover:'red'});expect(result.current.palette).toEqual(visualizerAlbumPalette(palette));expect(resolveArtworkTheme).toHaveBeenCalledTimes(1);
 rerender({id:12,cover:'green'});expect(result.current.palette).toBeUndefined();expect(result.current.pending).toBe(true);
 await waitFor(()=>expect(result.current.palette).toEqual(visualizerAlbumPalette(green)));expect(result.current.pending).toBe(false);
 rerender({id:10,cover:'red'});expect(result.current.palette).toEqual(visualizerAlbumPalette(palette));expect(resolveArtworkTheme).toHaveBeenCalledTimes(2);
});
it('updates replaced artwork and ignores the result from before invalidation',async()=>{
 let stale:(value:any)=>void=()=>{};const updated={...palette,swatches:['#222255','#6666aa','#aabbff']};
 vi.mocked(resolveArtworkTheme).mockReset().mockImplementationOnce(()=>new Promise(r=>{stale=r;})).mockResolvedValueOnce({palette:updated,path:'same-path'});
 const {result}=renderHook(()=>useVisualizerAlbumPalette(true,21,'same-path'));
 act(()=>window.dispatchEvent(new Event('musicos-artwork-invalidated')));
 await waitFor(()=>expect(result.current.palette).toEqual(visualizerAlbumPalette(updated)));
 await act(async()=>stale({palette,path:'same-path'}));expect(result.current.palette).toEqual(visualizerAlbumPalette(updated));
});
it('does not reuse embedded colors for another track without a known cover',async()=>{
 vi.mocked(resolveArtworkTheme).mockReset().mockResolvedValueOnce({palette,path:'embedded-31'}).mockImplementationOnce(()=>new Promise(()=>{}));
 const {result,rerender}=renderHook(({id})=>useVisualizerAlbumPalette(true,id,null),{initialProps:{id:31}});
 await waitFor(()=>expect(result.current.palette).toEqual(visualizerAlbumPalette(palette)));
 rerender({id:32});expect(result.current.palette).toBeUndefined();expect(resolveArtworkTheme).toHaveBeenLastCalledWith([null],32);
});
