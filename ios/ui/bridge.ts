import {invoke,convertFileSrc,isTauri} from '@tauri-apps/api/core';
export const native=isTauri();
export const assets=Object.fromEntries(Object.entries(import.meta.glob('../../assets/icons/ui/*.svg',{eager:true,query:'?url',import:'default'})).map(([path,url])=>[path.replace('../../',''),url as string]));
export function asset(path:string){return assets[path]??'';}
export function artwork(path:string|null|undefined){return !path?asset('assets/icons/ui/noCOVER.svg'):path.startsWith('data:')||path.startsWith('/assets/')?path:native?convertFileSrc(path):path;}
export const emptyPlayback={track:null,queue:[],queue_index:null,is_playing:false,position:0,volume:1,shuffle:false,repeat:'none',error:null,backend:'ios'};
export async function request(payload:Record<string,unknown>):Promise<any>{const result=await invoke<string>('plugin:companion|request',{payload});return JSON.parse(result);}
export async function call(command:string,args?:any):Promise<any>{
 switch(command){
 case 'lyrics':return request({action:'lyrics',id:args.request.trackId});
 case 'apple_enrichment':return {enabled:false,status:{connected:false},capabilities:{},matches:[],candidates:[]};
 case 'playback':return request({action:args?.action?.action??args?.action?.kind??'playback',...args?.action});
 case 'trackpad_feedback':return;
 case 'portrait_artwork':case 'artwork_rendition':return args.path;
 case 'depth_artwork':return null;
 case 'library_artwork':{const state=await request({action:'snapshot'});return state.tracks.find((t:any)=>t.id===args.trackId)?.cover_art_path??null;}
 case 'cache_artwork':return args.url;
 case 'artwork_palette':return request({action:'artwork_palette',path:args.path});
 default:throw new Error('This operation is available on the Mac');
 }
}
export function libraryArtwork(trackId:number){return call('library_artwork',{trackId});}
export async function localArtwork(url:string){return url;}
export function invalidateArtwork(){window.dispatchEvent(new Event('musicos-artwork-invalidated'));}
export function errorText(error:unknown):string{if(error instanceof Error)return error.message;if(typeof error==='string')return error;if(error&&typeof error==='object'&&'message' in error)return String(error.message);return 'Please try again.';}
export async function subscribe(){return()=>{};}
