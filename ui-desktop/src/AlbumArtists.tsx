import ArtistCredit from './ArtistCredit';
import type {Artist} from './types';
export default function AlbumArtists({artists,primaryId,loading,onOpen}:{artists:Artist[];primaryId:string;loading:Set<string>;onOpen:(artist:Artist)=>void}) {
 const ordered=[...artists].sort((a,b)=>Number(b.id===primaryId)-Number(a.id===primaryId));
 const credit=(artist:Artist)=><ArtistCredit key={artist.id} artist={artist} primary={artist.id===primaryId} loading={loading.has(artist.name)} onOpen={()=>onOpen(artist)}/>;
 return <div className="album-artist-credits" data-compact="true" data-many={ordered.length>6}><div className="album-credit-portraits" role="group" aria-label="Album artists">{ordered.length>6?<>{ordered.filter(a=>a.id===primaryId).map(credit)}<div className="album-guest-portraits" role="group" aria-label="Contributing artists">{ordered.filter(a=>a.id!==primaryId).map(credit)}</div></>:ordered.map(credit)}</div></div>;
}
