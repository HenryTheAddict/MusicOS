import type {Track} from './types';
export function trackCredits(track:Track):string[]{
 const feature=track.title.match(/(?:\(|\[|\s)\s*(?:feat\.?|ft\.?|featuring)\s+([^\)\]]+)/i)?.[1];
 const fields=[track.artist||'Unknown artist',...(feature?[feature]:[])];
 const names:string[]=[];
 for(const field of fields)for(const value of field.split(/;|&|\s+(?:featuring|feat\.?|ft\.?)\s+/i)){
  const name=value.trim();if(name&&!names.some(n=>n.toLocaleLowerCase()===name.toLocaleLowerCase()))names.push(name);
 }return names;
}
