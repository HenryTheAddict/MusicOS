import {describe,it,expect} from 'vitest';
import {ByteCache,GPU_BUDGET,reserveTexture,registerTexture,textureBytes} from '../src/depthCache';
const settle=()=>new Promise(resolve=>setTimeout(resolve,0));
describe('shared depth residency',()=>{
 it('coalesces pending loads and evicts the least recently used unpinned bytes',async()=>{
  const cache=new ByteCache<number>(10,n=>n);let finish!:(n:number)=>void;
  const pending=new Promise<number>(resolve=>finish=resolve);cache.set('visible',pending);cache.pin('visible',true);expect(cache.get('visible')).toBe(pending);finish(6);await settle();
  cache.set('old',Promise.resolve(4));await settle();cache.set('new',Promise.resolve(4));await settle();expect(cache.get('old')).toBeUndefined();expect(await cache.get('visible')).toBe(6);expect(cache.bytes).toBe(10);
  cache.pin('visible',false);cache.set('next',Promise.resolve(6));await settle();expect(cache.get('visible')).toBeUndefined();expect(cache.bytes).toBeLessThanOrEqual(10);
 });
 it('retries failed preparation and does not poison a later replacement',async()=>{
  const cache=new ByteCache<number>(10,n=>n);cache.set('art',Promise.reject(new Error('decode')));await settle();expect(cache.get('art')).toBeUndefined();cache.set('art',Promise.resolve(5));await settle();expect(await cache.get('art')).toBe(5);
 });
 it('shares one GPU budget, preserves visible textures and evicts offscreen textures',()=>{
  let evicted=false;const a=registerTexture({bytes:GPU_BUDGET/2,used:1,pinned:()=>true,evict:()=>{throw new Error('visible eviction');}});
  const b=registerTexture({bytes:GPU_BUDGET/2,used:2,pinned:()=>false,evict:()=>{evicted=true;}});
  try{expect(reserveTexture(GPU_BUDGET/2)).toBe(true);expect(evicted).toBe(true);expect(textureBytes()).toBe(GPU_BUDGET/2);expect(reserveTexture(GPU_BUDGET)).toBe(false);}finally{a();b();}expect(textureBytes()).toBe(0);
 });
});
