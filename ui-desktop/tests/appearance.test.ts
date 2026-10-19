import {expect,test} from 'vitest';import {appearance,themePalette,contrast,readableInk,defaultAppearance} from '../src/appearance';import type {Preferences} from '../src/types';
test('old appearance migrates to light and keeps artwork choice',()=>{expect(appearance({coverflow_colors:true} as Preferences)).toMatchObject({mode:'light',source:'artwork'});});
test('artwork mode preserves the curated album surfaces instead of whitening them again',()=>{
 const art={background:'#c8ded9',blob:'#55937a',accent:'#20574d',text:'#12202a'};
 const p=themePalette({...defaultAppearance,mode:'light',source:'artwork'},art,false)!;
 expect(p.background).toBe(art.background);expect(contrast(p.text,p.blob)).toBeGreaterThanOrEqual(4.8);expect(art.blob).toBe('#55937a');
 const red=themePalette({...defaultAppearance,mode:'light',source:'artwork'},{background:'#ecc4c4',blob:'#bc7777',accent:'#802525',text:'#12202a'},false)!;
 expect(red.background).not.toBe(p.background);expect(red.accent).not.toBe(p.accent);
 const untinted=themePalette({...defaultAppearance,mode:'light',source:'artwork',tint:0},art,false)!;
 expect(untinted.background).toBe('#f7f9fc');expect(art.background).toBe('#c8ded9');
});
test('curated light and dark colors keep text and selected controls readable at extremes',()=>{for(const mode of ['light','dark'] as const)for(const source of ['artwork','custom','blue'] as const)for(const accent of ['#ffffff','#000000','#ff00ff','#00ff00','#0000ff'])for(const saturation of [0,100,150]){const palette=themePalette({...defaultAppearance,mode,source,accent,saturation,tint:40},{background:accent,blob:accent,accent,text:'#12202a'},false);expect(palette).not.toBeNull();expect(contrast(palette!.text,palette!.background)).toBeGreaterThanOrEqual(4.5);expect(contrast(palette!.text,palette!.blob)).toBeGreaterThanOrEqual(4.5);expect(contrast(readableInk(palette!.accent),palette!.accent)).toBeGreaterThanOrEqual(4.5);}});

test('dark solid surfaces carry bounded artwork color, respect tint, and preserve the palette',()=>{
 const surfaces=[];
 for(const accent of ['#ad4110','#185a9b','#1d8160']){
  const art={background:accent,blob:accent,accent,text:'#000000'};
  const neutral=themePalette({...defaultAppearance,mode:'dark',source:'artwork',tint:0},art,false)!;
  expect(neutral.background).toBe('#202124');expect(neutral.blob).toBe('#2b2d32');
  const p=themePalette({...defaultAppearance,mode:'dark',source:'artwork',tint:20},art,false)!;
  expect(p.background).not.toBe(neutral.background);expect(p.blob).not.toBe(neutral.blob);
  expect(contrast(p.text,p.background)).toBeGreaterThan(10);expect(contrast(p.text,p.blob)).toBeGreaterThan(8);
  expect(contrast(p.accent,p.blob)).toBeGreaterThanOrEqual(4.5);expect(art.accent).toBe(accent);surfaces.push(p.background);
 }
 expect(new Set(surfaces).size).toBe(3);
});
test('dark red artwork is chromatic even when its absolute RGB range is small',()=>{
 const p=themePalette({...defaultAppearance,mode:'light',source:'artwork'},{accent:'#210908',background:'#bfa8a7',blob:'#a07f7e',text:'#12202a'},false)!;
 expect(p.accent).not.toBe('#210908');expect(parseInt(p.accent.slice(1,3),16)).toBeGreaterThan(parseInt(p.accent.slice(3,5),16)*2);expect(p.background).toBe('#bfa8a7');expect(contrast(p.accent,'#ffffff')).toBeGreaterThanOrEqual(4.5);
});
test('monochrome artwork gets a clean accent without tinting neutral dark surfaces',()=>{for(const gray of ['#222222','#808482','#dddddd']){const art={background:gray,blob:gray,accent:gray,text:'#111111'};const p=themePalette({...defaultAppearance,mode:'dark',source:'artwork'},art,false)!;expect(p.background).toBe('#202124');expect(p.blob).toBe('#2b2d32');expect(p.accent).not.toBe(gray);expect(contrast(p.accent,p.background)).toBeGreaterThanOrEqual(4.5);expect(art.accent).toBe(gray);}});

test('high saved saturation cannot turn artwork control accents into laser colors',()=>{
 const art={accent:'#210908',background:'#bfa8a7',blob:'#a07f7e',text:'#12202a'};
 for(const saturation of [100,125,150]){const p=themePalette({...defaultAppearance,mode:'light',source:'artwork',saturation},art,false)!;const channels=[1,3,5].map(i=>parseInt(p.accent.slice(i,i+2),16));expect(Math.max(...channels)).toBeLessThanOrEqual(168);expect((Math.max(...channels)-Math.min(...channels))/Math.max(...channels)).toBeLessThanOrEqual(.73);expect(contrast(p.accent,'#ffffff')).toBeGreaterThanOrEqual(4.5);}
 expect(art.accent).toBe('#210908');
});

test('muted green and monochrome artwork never acquire a default blue accent',()=>{
 for(const mode of ['light','dark'] as const)for(const saturation of [100,150]){
  for(const accent of ['#252e27','#292929']){
   const art={accent,background:'#c5d7c9',blob:'#b6c2b9',text:'#12202a'};
   const p=themePalette({...defaultAppearance,mode,source:'artwork',saturation,tint:20},art,false)!;
   const channels=[1,3,5].map(i=>parseInt(p.accent.slice(i,i+2),16));
   if(accent==='#252e27')expect(channels[2]).toBeLessThanOrEqual(channels[1]);
   else expect(Math.max(...channels)-Math.min(...channels)).toBeLessThan(30);
   expect(contrast(readableInk(p.accent),p.accent)).toBeGreaterThanOrEqual(4.5);
   expect(art.accent).toBe(accent);
  }
 }
});
