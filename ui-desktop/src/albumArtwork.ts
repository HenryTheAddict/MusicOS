import type {Library} from './types';
const covers=new Map<number,string|null>();
export function registerAlbumArtwork(library:Library){covers.clear();for(const album of library.albums)for(const id of album.track_ids)covers.set(id,album.cover);}
export function albumArtwork(id?:number){return id===undefined?undefined:covers.get(id);}
