import {useEffect,useRef,useState} from 'react';
import {resolveArtworkTheme} from './artworkTheme';
import type {Palette} from './types';
import type {AlbumPalette} from './visualizer';

/** Use artwork colors, independently of the user's interface theme or tint. */
export function visualizerAlbumPalette(palette:Palette):AlbumPalette|undefined{
 const colors=palette.swatches?.length===3?palette.swatches:[palette.accent,palette.blob,palette.background];
 if(colors.some(color=>!/^#[\da-f]{6}$/i.test(color)))return undefined;
 return colors.map(color=>[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)/255)) as AlbumPalette;
}
export function useVisualizerAlbumPalette(enabled:boolean,trackId:number|undefined,cover:string|null|undefined){
 const [resolved,setResolved]=useState<{key:string;palette:AlbumPalette|undefined}|null>(null);
 const cache=useRef(new Map<string,AlbumPalette|undefined>()),[revision,setRevision]=useState(0);
 // Tracks sharing a cover share its colors. Re-keying by song briefly removed
 // the palette on every skip, even when that exact artwork was already ready.
 const key=JSON.stringify([revision,cover?['cover',cover]:['embedded',trackId]]);
 useEffect(()=>{const invalidate=()=>{cache.current.clear();setResolved(null);setRevision(n=>n+1);};window.addEventListener('musicos-artwork-invalidated',invalidate);return()=>window.removeEventListener('musicos-artwork-invalidated',invalidate);},[]);
 useEffect(()=>{
  if(!enabled||cache.current.has(key))return;
  let alive=true;
  void resolveArtworkTheme([cover],trackId).then(result=>{
   if(alive){const palette=result?visualizerAlbumPalette(result.palette):undefined;cache.current.set(key,palette);setResolved({key,palette});}
  }).catch(()=>{if(alive)setResolved({key,palette:undefined});});
  return()=>{alive=false;};
 },[enabled,key,trackId,cover]);
 // An old album must never tint a new track while its artwork is loading.
 const ready=cache.current.has(key)||resolved?.key===key;
 return {palette:enabled?(cache.current.has(key)?cache.current.get(key):resolved?.key===key?resolved.palette:undefined):undefined,pending:enabled&&!ready};
}
