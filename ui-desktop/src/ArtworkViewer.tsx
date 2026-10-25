import {useEffect,useRef,useState} from 'react';
import {useDepthArtwork} from './depthArtwork';
import {artwork} from './bridge';
import {artworkRendition} from './useArtworkRendition';
type Photo={src:string;title:string;path?:string;version?:string;opener:HTMLElement};
export function openArtwork(element:HTMLElement,title:string){
 const image=element.querySelector<HTMLImageElement>('img');
 if(image&&!image.classList.contains('placeholder'))window.dispatchEvent(new CustomEvent<Photo>('musicos-expand-artwork',{detail:{src:image.currentSrc||image.src,title,path:image.dataset.originalArtworkPath||image.dataset.artworkPath,version:image.dataset.artworkVersion??'0',opener:element}}));
}
export default function ArtworkViewer({depth=false}:{depth?:boolean}){
 const stage=useRef<HTMLDivElement>(null);
 const dialog=useRef<HTMLDialogElement>(null),[photo,setPhoto]=useState<Photo|null>(null);
 const returnTo=useRef<HTMLElement|null>(null),isOpen=!!photo;
 const [large,setLarge]=useState<{photo:Photo;src:string}|null>(null);
 const src=photo&&(large?.photo===photo?large.src:photo.src);
 useEffect(()=>{
  if(!photo?.path)return;let alive=true;
  void artworkRendition(photo.path,2048,Number(photo.version)||0).then(async path=>{
   if(!alive)return;const src=artwork(path);const decoded=new Image();decoded.src=src;await decoded.decode();
   if(alive)setLarge({photo,src});
  }).catch(()=>{});
  return()=>{alive=false;};
 },[photo]);
 useDepthArtwork(stage,stage,depth&&!!photo);
 useEffect(()=>{const show=(e:Event)=>{const next=(e as CustomEvent<Photo>).detail;returnTo.current=next.opener;setPhoto(next);};window.addEventListener('musicos-expand-artwork',show);return()=>window.removeEventListener('musicos-expand-artwork',show);},[]);
 useEffect(()=>{if(!isOpen)return;const node=dialog.current;if(!node)return;node.showModal();const dismiss=()=>node.close();window.addEventListener('popstate',dismiss);return()=>{window.removeEventListener('popstate',dismiss);node.close();if(returnTo.current?.isConnected)returnTo.current.focus({preventScroll:true});};},[isOpen]);
 return <dialog ref={dialog} className="artwork-viewer" aria-label={photo?`${photo.title} artwork`:'Artwork'} onClose={()=>setPhoto(null)} onClick={e=>{if(e.target===e.currentTarget)e.currentTarget.close();}}><button autoFocus aria-label="Close artwork" onClick={()=>dialog.current?.close()}>×</button>{photo&&<><img className="viewer-backdrop" src={src!} alt="" aria-hidden="true" draggable={false}/><figure><div className="viewer-depth-stage" ref={stage}><div className="viewer-artwork"><img className="cover" data-artwork-path={photo.path} data-artwork-version={photo.version??'0'} src={src!} alt={photo.title} draggable={false}/></div></div><figcaption>{photo.title}</figcaption></figure></>}</dialog>;
}
