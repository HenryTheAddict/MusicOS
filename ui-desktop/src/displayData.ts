import {call} from './bridge';
/** Display data only. Never use this cache for credentials or mutation/preflight authorization. */
type Entry={value?:unknown;pending?:Promise<unknown>;dirty:boolean;generation:number};
const entries=new Map<string,Entry>();
export const displayKey=(command:string,args:unknown)=>command+':'+JSON.stringify(args??{});
export async function displayData<K extends Parameters<typeof call>[0]>(command:K,args?:Parameters<typeof call<K>>[1],fresh=false):Promise<Awaited<ReturnType<typeof call<K>>>>{
 const key=displayKey(command,args);let entry=entries.get(key);if(!entry){if(entries.size>=128){const oldest=[...entries].find(([,value])=>!value.pending);if(oldest)entries.delete(oldest[0]);}entry={dirty:true,generation:0};entries.set(key,entry);}
 if(entry.pending)return entry.pending as Promise<Awaited<ReturnType<typeof call<K>>>>;
 if(!fresh&&!entry.dirty&&entry.value!==undefined)return entry.value as Awaited<ReturnType<typeof call<K>>>;
 const target=entry,generation=entry.generation;
 const pending=call(command,args).then(value=>{target.value=value;target.dirty=target.generation!==generation;return value;}).finally(()=>{target.pending=undefined;});
 target.pending=pending;const value=await pending;if(target.dirty)return displayData(command,args);return value as Awaited<ReturnType<typeof call<K>>>;
}
export function invalidateDisplay(command?:string){for(const [key,entry] of entries)if(!command||key.startsWith(command+':')){entry.dirty=true;entry.generation++;}window.dispatchEvent(new CustomEvent('musicos-display-invalidated',{detail:command}));}
export type LibraryChanged={reason?:'metadata'|'artwork'|'sources'|'analysis'|'favorites'|'collections';track_ids?:number[];favorite_updates?:{track_id:number;favorited:boolean}[]};

/** Read the last display value synchronously while a visible page revalidates it. */
export function peekDisplayData<K extends Parameters<typeof call>[0]>(command:K,args?:Parameters<typeof call<K>>[1]):Awaited<ReturnType<typeof call<K>>>|undefined{return entries.get(displayKey(command,args))?.value as Awaited<ReturnType<typeof call<K>>>|undefined;}
