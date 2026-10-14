import {useEffect,useRef,useState} from 'react';
import {useDepthArtwork} from './depthArtwork';
type Photo={src:string;title:string;path?:string;version?:string;opener:HTMLElement};
export function openArtwork(element:HTMLElement,title:string){
 const image=element.querySelector<HTMLImageElement>('img');
 if(image&&!image.classList.contains('placeholder'))window.dispatchEvent(new CustomEvent<Photo>('musicos-expand-artwork',{detail:{src:image.dataset.fullSrc||image.currentSrc,title,path:image.dataset.originalArtworkPath||image.dataset.artworkPath,version:image.dataset.artworkVersion??'0',opener:element}}));
}
export default function ArtworkViewer({depth=false}:{depth?:boolean}){
 const stage=useRef<HTMLDivElement>(null);
 const dialog=useRef<HTMLDialogElement>(null),[photo,setPhoto]=useState<Photo|null>(null);
 useDepthArtwork(stage,stage,depth&&!!photo);
 useEffect(()=>{const show=(e:Event)=>setPhoto((e as CustomEvent<Photo>).detail);window.addEventListener('musicos-expand-artwork',show);return()=>window.removeEventListener('musicos-expand-artwork',show);},[]);
 useEffect(()=>{if(!photo)return;const node=dialog.current;if(!node)return;node.showModal();const dismiss=()=>node.close();window.addEventListener('popstate',dismiss);return()=>{window.removeEventListener('popstate',dismiss);node.close();if(photo.opener.isConnected)photo.opener.focus({preventScroll:true});};},[photo]);
 return <dialog ref={dialog} className="artwork-viewer" aria-label={photo?`${photo.title} artwork`:'Artwork'} onClose={()=>setPhoto(null)} onClick={e=>{if(e.target===e.currentTarget)e.currentTarget.close();}}><button autoFocus aria-label="Close artwork" onClick={()=>dialog.current?.close()}>×</button>{photo&&<><img className="viewer-backdrop" src={photo.src} alt="" aria-hidden="true" draggable={false}/><figure><div className="viewer-depth-stage" ref={stage}><div className="viewer-artwork"><img className="cover" data-artwork-path={photo.path} data-artwork-version={photo.version??'0'} src={photo.src} alt={photo.title} draggable={false}/></div></div><figcaption>{photo.title}</figcaption></figure></>}</dialog>;
}
