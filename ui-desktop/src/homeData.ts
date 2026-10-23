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
/** Local seed ranking for new audio mixes; generation stays behind an explicit preview. */
export function homeMixIdeas(library:Library,activity:HomeActivity|undefined,current?:number){
 const recent=new Map([...new Set(activity?.recent??[])].map((id,index)=>[id,index]));
 const frequent=new Map((activity?.frequent??[]).map(row=>[row.id,row.plays]));
 const maximum=Math.max(1,...frequent.values());
 const candidates=library.tracks.filter(track=>track.available).map(track=>({track,
  score:(track.id===current?4:0)+(recent.has(track.id)?1.5/(1+recent.get(track.id)!*.35):0)+(frequent.get(track.id)??0)/maximum+(track.favorited ? .35 : 0),
  reason:track.id===current?'From what’s playing':recent.has(track.id)?'From your recent listening':frequent.has(track.id)?'From your most played':'A favorite to build on',
 })).filter(row=>row.score>0);
 const selected:typeof candidates=[];
 while(selected.length<3&&candidates.length){candidates.sort((a,b)=>{
  const score=(row:typeof a)=>row.score-selected.reduce((n,old)=>n+(old.track.artist===row.track.artist ? .6 : 0)+(old.track.album===row.track.album ? .8 : 0),0);
  return score(b)-score(a)||a.track.id-b.track.id;
 });selected.push(candidates.shift()!);}
 return selected;
}
