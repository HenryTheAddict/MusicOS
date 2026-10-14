import {expect,test} from 'vitest';
import {sparkNames,seedVibeTitle,hexHue} from '../src/sparkSuggestions';
test('vibe names use the chosen mood and a clean song-title-esque suggestion',()=>{expect(sparkNames('November (feat. Artist)','Melancholic',245)).toContain('November-esque');expect(sparkNames('November','Melancholic',245)).toEqual(['November-esque']);expect(seedVibeTitle('Song [Explicit]')).toBe('Song');expect(sparkNames(undefined,null,55)).toEqual([]);expect(sparkNames('X'.repeat(200),null)[0].length).toBeLessThanOrEqual(120);});
test('cover colors map to a bounded hue independently of the current interface palette',()=>{expect(hexHue('#ff0000')).toBe(0);expect(hexHue('#0000ff')).toBe(240);expect(hexHue('invalid')).toBeNull();});
