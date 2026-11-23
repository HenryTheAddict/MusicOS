import {test,expect} from 'vitest';
import {trackCredits,albumArtists} from '../src/credits';
import type {Track,Library} from '../src/types';
test('spaced plus credits become individual contributors and literal plus names remain intact',()=>{expect(trackCredits({artist:'PinkPuddleworks + Confetti Badgers',title:'Unauthorized Teacup Parade'} as Track)).toEqual(['PinkPuddleworks','Confetti Badgers']);expect(trackCredits({artist:'C++',title:'Song (feat. Guest)'} as Track)).toEqual(['C++','Guest']);});
test('album artists exclude guest-only identities and keep co-album artists',()=>{const library={albums:[{artist:'A + B'}],artists:[{id:'a',name:'A'},{id:'b',name:'B'},{id:'guest',name:'Guest'}]} as Library;expect(albumArtists(library).map(a=>a.id)).toEqual(['a','b']);});
