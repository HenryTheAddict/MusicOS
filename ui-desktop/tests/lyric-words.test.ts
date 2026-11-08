import {it,expect} from 'vitest';
import {lyricWordSegments,lyricWordEnd,lyricWordActive,lyricWordState,wordProgress,wordGlow} from '../src/lyricWords';
import type {LyricLine} from '../src/lyrics';
it('renders exact source punctuation, spaces and Unicode rather than provider labels',()=>{
 const line={text:'“Hello,”  世界! — مرحباً',start_ms:0,words:[{text:'Hello',start_ms:0},{text:'世界',start_ms:100},{text:'مرحباً ',start_ms:200}]};const segments=lyricWordSegments(line)!;
 expect(segments.map(s=>s.text).join('')).toBe(line.text);expect(segments.filter(s=>s.word!==undefined).map(s=>s.text)).toEqual(['Hello','世界','مرحباً']);
});
it('keeps syllables adjacent and repeated words attached in source order',()=>{
 const line={text:'hello hello',start_ms:0,words:[{text:'hel',start_ms:0},{text:'lo',start_ms:100},{text:'hello',start_ms:200}]};expect(lyricWordSegments(line)).toEqual([{text:'hel',word:0},{text:'lo',word:1},{text:' '},{text:'hello',word:2}]);
});
it('incompatible word labels leave the original text to line timing',()=>{expect(lyricWordSegments({text:'Original words',start_ms:0,words:[{text:'Different',start_ms:0}]})).toBeNull();expect(lyricWordSegments({text:'  ',start_ms:0,words:[{text:' ',start_ms:0}]})).toBeNull();});
const line:LyricLine={text:'one two',start_ms:1000,words:[{text:'one',start_ms:1000,end_ms:1200},{text:'two',start_ms:1800,end_ms:2200}]};
it('explicit word endings clear highlights in gaps and backward seeks restore them',()=>{expect(lyricWordState(line,1100)).toBe('10');expect(lyricWordState(line,1200)).toBe('20');expect(lyricWordActive(line,0,1500)).toBe(false);expect(lyricWordState(line,1900)).toBe('21');expect(lyricWordState(line,900)).toBe('00');});
it('uses only available ends, with no invented final word duration',()=>{const open={text:'one two',start_ms:0,words:[{text:'one',start_ms:0},{text:'two',start_ms:1000}]};expect(lyricWordEnd(open,0)).toBe(1000);expect(lyricWordEnd(open,1)).toBeUndefined();expect(lyricWordEnd({...open,end_ms:3000},1)).toBe(3000);expect(wordProgress(1000,undefined,1000)).toBe(1);expect(wordProgress(1000,2000,1500)).toBe(.5);expect(wordProgress(1000,2000,2500)).toBe(1);});
it('overlapping words remain active together',()=>{const overlap={...line,words:[{text:'one',start_ms:1000,end_ms:2100},{text:'two',start_ms:1800,end_ms:2200}]};expect(lyricWordState(overlap,1900)).toBe('11');expect(lyricWordState(overlap,2100)).toBe('21');});

it('word light eases across boundaries without changing real highlight timing',()=>{expect(wordProgress(1000,undefined,999)).toBe(0);expect(wordGlow(1000,1500,999)).toBe(0);expect(wordGlow(1000,1500,1000)).toBe(0);expect(wordGlow(1000,1500,1099)).toBeGreaterThan(.4);expect(wordGlow(1000,1500,1500)).toBeGreaterThan(0);expect(wordGlow(1000,1500,1600)).toBe(0);expect(wordGlow(1000,1500,999)).toBe(0);});
it('simultaneous real word starts share their next valid end instead of snapping fully lit',()=>{
 const line={text:'One two three',start_ms:1000,end_ms:3000,words:[{text:'One',start_ms:1000},{text:'two',start_ms:1000},{text:'three',start_ms:2000,end_ms:3000}]};expect(lyricWordEnd(line,0)).toBe(2000);expect(lyricWordEnd(line,1)).toBe(2000);
});
