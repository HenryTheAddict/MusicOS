import {paletteFromPixels} from '../../ui-desktop/src/artworkPixels';
import type {Palette} from '../../ui-desktop/src/types';
import {artwork,native,request} from './bridge';
const cache=new Map<string,Promise<Palette|null>>();
export function coverPalette(path:string){let job=cache.get(path);if(!job){job=native?request({action:'artwork_palette',path}).catch(()=>null):new Promise<Palette|null>(resolve=>{const image=new Image();image.onload=()=>{try{const canvas=document.createElement('canvas');canvas.width=canvas.height=64;const context=canvas.getContext('2d',{willReadFrequently:true});if(!context){resolve(null);return;}context.drawImage(image,0,0,64,64);resolve(paletteFromPixels(context.getImageData(0,0,64,64).data));}catch{resolve(null);}};image.onerror=()=>resolve(null);image.src=artwork(path);});if(cache.size>=64)cache.delete(cache.keys().next().value!);cache.set(path,job);}return job;}
