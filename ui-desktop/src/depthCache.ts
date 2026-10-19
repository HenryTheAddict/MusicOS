/** Shared decoded-resource LRU. In-flight loads coalesce; visible resources are pinned. */
export class ByteCache<T>{
 private items=new Map<string,{promise:Promise<T>;bytes:number;pins:number;used:number}>();private clock=0;
 hits=0;misses=0;
 constructor(readonly budget:number,private size:(value:T)=>number){}
 get(key:string){const item=this.items.get(key);if(item){item.used=++this.clock;this.hits++;}return item?.promise;}
 set(key:string,promise:Promise<T>){const item={promise,bytes:0,pins:0,used:++this.clock};this.items.set(key,item);this.misses++;promise.then(value=>{if(this.items.get(key)!==item)return;item.bytes=this.size(value);this.trim();}).catch(()=>{if(this.items.get(key)===item)this.items.delete(key);});}
 delete(key:string){this.items.delete(key);}
 clear(){this.items.clear();}
 pin(key:string,on:boolean){const item=this.items.get(key);if(item)item.pins=Math.max(0,item.pins+(on?1:-1));if(!on)this.trim();}
 private trim(){let bytes=this.bytes;for(const [key,item]of [...this.items].sort((a,b)=>a[1].used-b[1].used)){if(bytes<=this.budget)break;if(item.pins||!item.bytes)continue;bytes-=item.bytes;this.items.delete(key);}}
 get bytes(){return [...this.items.values()].reduce((sum,item)=>sum+item.bytes,0);}
 get count(){return this.items.size;}
}
type Resident={bytes:number;used:number;pinned:()=>boolean;evict:()=>void};
const gpu=new Set<Resident>();export const GPU_BUDGET=128*1024*1024;
export function reserveTexture(bytes:number){let total=textureBytes();for(const item of [...gpu].sort((a,b)=>a.used-b.used)){if(total+bytes<=GPU_BUDGET)break;if(item.pinned())continue;total-=item.bytes;item.evict();gpu.delete(item);}return total+bytes<=GPU_BUDGET;}
export function registerTexture(item:Resident){gpu.add(item);return()=>gpu.delete(item);}
export function textureBytes(){return [...gpu].reduce((sum,item)=>sum+item.bytes,0);}
