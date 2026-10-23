import {usePortraitArtwork} from './portraitArtwork';
import {artwork} from './bridge';
import {PortraitLoading} from './components';
import type {Artist} from './types';

export default function ArtistCredit({artist,primary,loading,onOpen}:{artist:Artist;primary:boolean;loading:boolean;onOpen:()=>void}){
 const portrait=usePortraitArtwork(artist.portrait);
 return <button className={`artist-link ${primary?'album-primary-artist':'album-collaborator'}`} aria-label={`View artist ${artist.name}`} onClick={onOpen}>
  <span className="artist-avatar" aria-hidden="true">{artist.portrait?<img src={artwork(portrait?.display)} alt="" draggable={false}/>:artist.name.slice(0,1)}{!artist.portrait&&loading&&<PortraitLoading/>}</span>
  <span className="artist-credit-name">{artist.name}</span>
 </button>;
}
