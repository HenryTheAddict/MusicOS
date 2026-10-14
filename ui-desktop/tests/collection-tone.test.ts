import {expect,test} from 'vitest';
import {collectionPalette} from '../src/collectionTone';
test('cover colors determine glyphs without depending on the interface theme',()=>{
 const a=collectionPalette([220,40,20,255,20,200,50,255,40,30,210,255]);
 const b=collectionPalette([40,80,120,255,40,80,120,255]);
 expect(a.stops).toHaveLength(3);expect(new Set(a.stops).size).toBe(3);
 expect(a.tone).not.toBe(b.tone);
 for(const stop of a.stops)expect(Math.max(...stop.match(/\d+/g)!.map(Number))).toBeLessThanOrEqual(112);
});
test('transparent and white pixels do not tint the control canvas',()=>{
 expect(collectionPalette([255,255,255,255,255,0,0,0])).toEqual(collectionPalette([]));
});
