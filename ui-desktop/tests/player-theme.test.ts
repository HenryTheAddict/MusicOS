import {expect,test} from 'vitest';
import {themePalette,defaultAppearance,contrast} from '../src/appearance';
import {transportTones} from '../src/playerTheme';
test('two-tone transport follows the design hue and stays readable in both appearances',()=>{
 for(const mode of ['light','dark'] as const){
  const colors=[];
  for(const accent of ['#1e6950','#8d3232','#1b4f9d']){
   const p=themePalette({...defaultAppearance,mode,source:'custom',accent,tint:15},null,false)!;
   const tones=transportTones(p);colors.push(tones);
   expect(contrast(tones.fill,p.background)).toBeGreaterThanOrEqual(4.5);
   expect(contrast(tones.stroke,p.background)).toBeGreaterThanOrEqual(3);
   expect(contrast(tones.fill,tones.stroke)).toBeGreaterThanOrEqual(1.4);
  }
  expect(new Set(colors.map(c=>c.fill)).size).toBe(3);expect(new Set(colors.map(c=>c.stroke)).size).toBe(3);
 }
});
test('extreme custom themes keep a distinct outline and readable fill',()=>{
 for(const mode of ['light','dark'] as const)for(const accent of ['#ffffff','#000000','#00ff00','#ff00ff','#0000ff']){
  const p=themePalette({...defaultAppearance,mode,source:'custom',accent,saturation:150,tint:40},null,false)!;
  const tones=transportTones(p);
  expect(contrast(tones.fill,p.background)).toBeGreaterThanOrEqual(4.5);
  expect(contrast(tones.stroke,p.background)).toBeGreaterThanOrEqual(3);
  expect(tones.fill).not.toBe(tones.stroke);
 }
});
