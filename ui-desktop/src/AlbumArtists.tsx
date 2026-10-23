import ArtistCredit from './ArtistCredit';
import {Menu} from './components';
import type {Artist} from './types';

export default function AlbumArtists({artists,primaryId,loading,onOpen}:{artists:Artist[];primaryId:string;loading:Set<string>;onOpen:(artist:Artist)=>void}) {
 const primary=artists.find(artist=>artist.id===primaryId)??artists[0];
 const unique=[...new Map(artists.map(artist=>[artist.id,artist])).values()];
 const guests=unique.filter(artist=>artist.id!==primary?.id);
 const visible=guests.length>3?guests.slice(0,2):guests;
 return <div className="album-artist-credits" role="group" aria-label="Album artists">
  {primary&&<ArtistCredit artist={primary} primary loading={loading.has(primary.name)} onOpen={()=>onOpen(primary)}/>}
  {guests.length>0&&<div className="album-contributors" role="group" aria-label="Contributing artists">
   <span className="album-contributors-label">With</span>
   {visible.map(artist=><ArtistCredit key={artist.id} artist={artist} primary={false} loading={loading.has(artist.name)} onOpen={()=>onOpen(artist)}/>)}
   {guests.length>3&&<Menu label={`${guests.length-visible.length} more contributing artists`} items={guests.slice(visible.length).map(artist=>({label:artist.name,portrait:artist.portrait,icon:'Artist',onSelect:()=>onOpen(artist)}))}>+{guests.length-visible.length} artists</Menu>}
  </div>}
 </div>;
}
