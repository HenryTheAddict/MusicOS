import {it,expect} from 'vitest';
import {paletteFromPixels} from '../src/artworkPixels';
it('retains distinct actual artwork hues instead of copying a single dominant bin',()=>{
 const pixels=Array.from({length:100},(_,i)=>i<40?[20,22,25,255]:i<70?[180,32,45,255]:[28,104,162,255]).flat();const palette=paletteFromPixels(pixels)!;expect(new Set(palette.swatches).size).toBeGreaterThan(1);expect(palette.swatches!.some(c=>parseInt(c.slice(1,3),16)>100)).toBe(true);expect(palette.swatches!.some(c=>parseInt(c.slice(5,7),16)>100)).toBe(true);
});
it('handles transparent and monochrome artwork without inventing a blue accent',()=>{
 expect(paletteFromPixels([200,0,0,0])).toBeNull();const palette=paletteFromPixels(Array.from({length:40},(_,i)=>[i*5,i*5,i*5,255]).flat())!;for(const c of palette.swatches!){expect(c.slice(1,3)).toBe(c.slice(3,5));expect(c.slice(1,3)).toBe(c.slice(5,7));}
});
