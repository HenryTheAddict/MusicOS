import type {Track,Library,Artist} from './types';
export function trackCredits(track:Track):string[]{
 const feature=track.title.match(/(?:\(|\[|\s)\s*(?:feat\.?|ft\.?|featuring)\s+([^\)\]]+)/i)?.[1];
 const fields=[track.artist||'Unknown artist',...(feature?[feature]:[])];
 const names:string[]=[];
 for(const field of fields)for(const value of field.split(/;|&|\s+\+\s+|\s+(?:featuring|feat\.?|ft\.?)\s+/i)){
  const name=value.trim();if(name&&!names.some(n=>n.toLocaleLowerCase()===name.toLocaleLowerCase()))names.push(name);
 }return names;
}

export function albumArtists(library:Library):Artist[]{const names=new Set(library.albums.flatMap(album=>trackCredits({artist:album.artist,title:''} as Track)).map(name=>name.trim().toLocaleLowerCase()));return library.artists.filter(artist=>[artist.name,...(artist.credited_names??[])].some(name=>names.has(name.trim().toLocaleLowerCase())));}
