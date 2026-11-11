import type {Track} from '../../ui-desktop/src/types';

const names=new Intl.Collator(undefined,{numeric:true,sensitivity:'base'});
const text=(a:unknown,b:unknown)=>names.compare(String(a??''),String(b??''));
const number=(value:unknown,fallback:number)=>Number.isFinite(Number(value))&&Number(value)>0?Number(value):fallback;
const identity=(a:Track,b:Track)=>text(a.album_artist??a.artist,b.album_artist??b.artist)||text(a.album,b.album)||a.id-b.id;

/** Catalogue order is presentation only. Saved collection and queue sequences never pass through these sorts. */
export function songOrder(a:Track,b:Track){return text(a.title,b.title)||identity(a,b);}
export function albumTrackOrder(a:Track,b:Track){
 return number(a.disc_number,1)-number(b.disc_number,1)
  ||number(a.track_number,Number.MAX_SAFE_INTEGER)-number(b.track_number,Number.MAX_SAFE_INTEGER)
  ||songOrder(a,b);
}
export function orderedAlbums<T extends Track&{album_id?:string}>(tracks:readonly T[]):T[][]{
 const groups=new Map<string,T[]>();
 for(const track of tracks){
  const key=track.album_id??JSON.stringify([track.album,track.album_artist??track.artist,track.year]);
  const group=groups.get(key);if(group)group.push(track);else groups.set(key,[track]);
 }
 return [...groups.values()].map(group=>group.sort(albumTrackOrder)).sort((a,b)=>text(a[0].album,b[0].album)||identity(a[0],b[0])||text(a[0].album_id,b[0].album_id));
}
export function artistTrackOrder(a:Track,b:Track){return text(a.album,b.album)||albumTrackOrder(a,b);}
export function artistOrder(a:string,b:string){return text(a,b);}
