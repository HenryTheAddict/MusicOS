import ArtistCredit from './ArtistCredit';
import type {Artist} from './types';
export default function AlbumArtists({artists,primaryId,loading,onOpen}:{artists:Artist[];primaryId:string;loading:Set<string>;onOpen:(artist:Artist)=>void}) {
 const ordered=[...artists].sort((a,b)=>Number(b.id===primaryId)-Number(a.id===primaryId));
 return <div className="album-artist-credits" data-compact="true"><div className="album-credit-portraits" role="group" aria-label="Album artists">{ordered.map(artist=><ArtistCredit key={artist.id} artist={artist} primary={artist.id===primaryId} loading={loading.has(artist.name)} onOpen={()=>onOpen(artist)}/>)}</div></div>;
}
