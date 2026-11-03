import {useEffect,useState} from 'react';
import {resolveArtworkTheme} from './artworkTheme';
import type {Palette} from './types';
import type {AlbumPalette} from './visualizer';

/** Use artwork colors, independently of the user's interface theme or tint. */
export function visualizerAlbumPalette(palette:Palette):AlbumPalette|undefined{
 const colors=[palette.accent,palette.blob,palette.background];
 if(colors.some(color=>!/^#[\da-f]{6}$/i.test(color)))return undefined;
 return colors.map(color=>[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)/255)) as AlbumPalette;
}
export function useVisualizerAlbumPalette(enabled:boolean,trackId:number|undefined,cover:string|null|undefined){
 const [resolved,setResolved]=useState<{key:string;palette:AlbumPalette|undefined}|null>(null);
 const key=JSON.stringify([trackId,cover]);
 useEffect(()=>{
  if(!enabled)return;
  let alive=true;
  void resolveArtworkTheme([cover],trackId).then(result=>{
   if(alive)setResolved({key,palette:result?visualizerAlbumPalette(result.palette):undefined});
  }).catch(()=>{if(alive)setResolved({key,palette:undefined});});
  return()=>{alive=false;};
 },[enabled,key,trackId,cover]);
 // An old album must never tint a new track while its artwork is loading.
 return enabled&&resolved?.key===key?resolved.palette:undefined;
}
