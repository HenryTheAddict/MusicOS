import {describe,it,expect} from 'vitest';
import {collectionCovers,collectionCover} from '../src/CollectionDesign';
describe('Sketch collection artwork',()=>{
 it('includes the 32 manual and nine automatic covers, excluding legacy placeholders',()=>{expect(collectionCovers.filter(c=>c.id.startsWith('playlist/'))).toHaveLength(32);expect(collectionCovers.filter(c=>c.id.startsWith('auto/'))).toHaveLength(9);expect(new Set(collectionCovers.map(c=>c.id)).size).toBe(41);expect(collectionCover('playlist/Lake')).toBeTruthy();expect(collectionCover('auto/9')).toBeTruthy();expect(collectionCover('collage')).toBeUndefined();});
});
