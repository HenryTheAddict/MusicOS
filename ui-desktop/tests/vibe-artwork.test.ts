import {expect,test} from 'vitest';
import {encodeVibe,parseVibe,vibeCover,type VibeArt} from '../src/vibeArtwork';
const art:VibeArt={version:1,hue:210,energy:.8,shape:2,seed:321,variant:0,tempo:120};
test('sound-derived covers roundtrip, cache and vary only explicitly',()=>{const choice=encodeVibe(art);expect(parseVibe(choice)).toEqual(art);const cover=vibeCover(choice);expect(cover).toContain('data:image/svg+xml');expect(vibeCover(choice)).toBe(cover);expect(vibeCover(encodeVibe({...art,hue:247,variant:1}))).not.toBe(cover);expect(parseVibe('vibe/bad')).toBeNull();expect(parseVibe(encodeVibe({...art,hue:400}))).toBeNull();});
test('new cover variations are deterministic and keep legacy version-one covers stable',()=>{
 const old=vibeCover(encodeVibe(art));const versions=[0,1,2,3].map(variant=>vibeCover(encodeVibe({...art,version:2,variant})));
 expect(new Set(versions).size).toBe(4);expect(vibeCover(encodeVibe(art))).toBe(old);expect(parseVibe(encodeVibe({...art,seed:-1}))).toBeNull();
});
