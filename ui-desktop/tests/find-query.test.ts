import {expect,test} from 'vitest';
import {compileListFind} from '../src/findQuery';
const song={title:'Déjà Teacup (feat. Guest)',artist:'Moonspindle; Another Artist',album:'Hush, We’re Dreaming in Teacups',genre:'Dream Pop',year:2011,favorited:true,available:true};
test('unqualified terms combine across fields with accent and punctuation normalization',()=>{
 expect(compileListFind('deja moonspindle dreaming')(song)).toBe(true);expect(compileListFind('dreaming missing')(song)).toBe(false);expect(compileListFind('')(song)).toBe(true);
});
test('quoted phrases, qualified fields, negative terms and year ranges stay local',()=>{
 expect(compileListFind('artist:Moonspindle album:"were dreaming" year:2010-2012 -title:live')(song)).toBe(true);
 expect(compileListFind('title:Moonspindle')(song)).toBe(false);expect(compileListFind('"pop dream"')(song)).toBe(false);expect(compileListFind('-artist:Moonspindle')(song)).toBe(false);expect(compileListFind('year:2012')(song)).toBe(false);
 expect(compileListFind('album:"hush')(song)).toBe(true);
});
test('favorites and availability predicates do not use missing fields as evidence',()=>{
 expect(compileListFind('is:favorite is:AVAILABLE')(song)).toBe(true);expect(compileListFind('-is:liked')(song)).toBe(false);expect(compileListFind('is:unavailable')(song)).toBe(false);expect(compileListFind('is:favorite')({title:'favorite'})).toBe(false);
});
test('small spelling mistakes are tolerated only on longer unquoted words',()=>{
 expect(compileListFind('draming')(song)).toBe(true);expect(compileListFind('Opneing')({title:'Opening'})).toBe(true);expect(compileListFind('"draming"')(song)).toBe(false);expect(compileListFind('m84')(song)).toBe(false);
});
test('matching leaves order, duplicate identities and input records untouched',()=>{
 const items=[{...song,id:9},{...song,title:'Something else',id:2},{...song,id:9}];const saved=JSON.stringify(items);expect(items.filter(compileListFind('deja')).map(item=>item.id)).toEqual([9,9]);expect(JSON.stringify(items)).toBe(saved);
});
