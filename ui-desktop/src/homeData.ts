import type {Album,HomeActivity,Library} from './types';
/** Rank edition identities, never album titles; unavailable songs cannot keep a shelf alive. */
export function homeMusic(library:Library,activity:HomeActivity|undefined){
 const tracks=new Map(library.tracks.filter(t=>t.available).map(t=>[t.id,t]));
 const played=new Map((activity?.frequent??[]).filter(r=>r.plays>0&&tracks.has(r.id)).map(r=>[r.id,r.plays]));
 const rotation=library.albums.map(album=>({album,plays:album.track_ids.reduce((n,id)=>n+(played.get(id)??0),0)})).filter(r=>r.plays>0).sort((a,b)=>b.plays-a.plays||a.album.title.localeCompare(b.album.title)||a.album.id.localeCompare(b.album.id)).slice(0,6).map(r=>r.album);
 const recent=[...new Set(activity?.recent??[])].flatMap(id=>{const track=tracks.get(id);return track?[track]:[];}).slice(0,6);
 const explore:Album[]=library.albums.filter(a=>a.track_ids.some(id=>tracks.has(id))).slice(0,6);
 return {rotation,recent,explore};
}
