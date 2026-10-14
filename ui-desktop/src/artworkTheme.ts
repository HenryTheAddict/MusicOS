import { call, libraryArtwork, localArtwork } from "./bridge";
import type { Palette } from "./types";
const palettes=new Map<string,Promise<Palette>>();
export function clearThemeCache(){palettes.clear();}
export async function resolveArtworkTheme(paths:(string|null|undefined)[], trackId?:number|number[]):Promise<{palette:Palette;path:string}|null> {
  async function read(path:string) {
    let request=palettes.get(path);
    if(!request){
      request=(path.startsWith("https://")?localArtwork(path):Promise.resolve(path)).then(local=>call("artwork_palette",{path:local}));
      palettes.set(path,request);request.catch(()=>palettes.delete(path));
    }
    return {palette:await request,path};
  }
  for(const path of [...new Set(paths.filter((p):p is string=>!!p))]) {
    try{return await read(path);}catch{/* Try the next honest artwork source. */}
  }
  // Visible artwork can be embedded in a local track rather than pre-indexed.
  const tracks=Array.isArray(trackId)?trackId:trackId===undefined?[]:[trackId];
  for(const id of tracks)try{const path=await libraryArtwork(id);if(path)return await read(path);}catch{}
  return null;
}
