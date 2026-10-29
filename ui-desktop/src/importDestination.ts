import type {Library,Source} from './types';
/** Dropped files join the largest available library, with a stable tie break. */
export function dropImportSource(library:Pick<Library,'sources'|'tracks'>):Source|undefined{
 return library.sources.filter(source=>source.enabled!==false&&source.available).map(source=>({source,count:source.song_count??library.tracks.filter(track=>track.path.startsWith(source.path.replace(/\/$/,'')+'/')).length})).filter(item=>item.count>0).sort((a,b)=>b.count-a.count||a.source.id-b.source.id)[0]?.source;
}
