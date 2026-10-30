import {animateOwned} from './presentationMotion';
import {useLayoutEffect,type RefObject} from 'react';
import {smoothPath} from './smoothCorners';

type CoverKind='album'|'artist';
type CoverOrigin={kind:CoverKind;id:string;url:string;rect:DOMRect;returning:boolean;at:number};
let origin:CoverOrigin|null=null;
let toolbarExit:HTMLElement|null=null;
const reduced=()=>matchMedia('(prefers-reduced-motion: reduce)').matches;
/** Retain only the outgoing bar's pixels; navigation and layout change immediately. */
export function captureLibraryToolbar(root:HTMLElement|null){
 toolbarExit?.remove();toolbarExit=null;if(reduced())return;
 const toolbar=root?.closest('.content-column')?.querySelector<HTMLElement>(':scope > .library-toolbar:not(.library-toolbar-exit)');
 if(!toolbar)return;
 const box=toolbar.getBoundingClientRect(),copy=toolbar.cloneNode(true) as HTMLElement;
 copy.querySelectorAll('[popover]').forEach(node=>node.remove());
 copy.removeAttribute('id');copy.querySelectorAll('[id]').forEach(node=>node.removeAttribute('id'));
 copy.classList.add('library-toolbar-exit');copy.setAttribute('aria-hidden','true');copy.inert=true;
 Object.assign(copy.style,{left:`${box.left}px`,top:`${box.top}px`,width:`${box.width}px`,height:`${box.height}px`});
 toolbarExit=copy;
}
function snapshot(kind:CoverKind,id:string,surface:HTMLElement|null|undefined,returning:boolean){
 origin=null;if(!surface||reduced())return;
 const moving=document.querySelector<HTMLImageElement>('.album-cover-transfer');
 if(moving?.dataset.coverKind===kind&&moving.dataset.albumId===id){origin={kind,id,url:moving.src,rect:moving.getBoundingClientRect(),returning,at:performance.now()};return;}
 const image=surface.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');
 if(!image?.complete||!image.naturalWidth)return;
 const rect=image.getBoundingClientRect();
 if(!rect.width||!rect.height||rect.bottom<=0||rect.top>=innerHeight)return;
 origin={kind,id,url:image.currentSrc||image.src,rect,returning,at:performance.now()};
}
export function coverTransferPending(){return !!origin&&performance.now()-origin.at<750;}
export function captureAlbumCover(id:string,surface?:HTMLElement){snapshot('album',id,surface,false);}
export function captureAlbumReturn(root:HTMLElement|null){const album=root?.querySelector<HTMLElement>('.album-detail[data-album-id]');snapshot('album',album?.dataset.albumId??'',album,true);}

export function captureArtistCover(id:string,surface?:HTMLElement){snapshot('artist',id,surface,false);}
export function captureArtistReturn(root:HTMLElement|null){const artist=root?.querySelector<HTMLElement>('.artist-detail[data-artist-id]');snapshot('artist',artist?.dataset.artistId??'',artist,true);}

/** Navigation owns these short effects; data refresh and artwork decode never replay them. */
export function useAlbumReveal(root:RefObject<HTMLElement|null>,location:string,ready:boolean){
 useLayoutEffect(()=>{
  const page=root.current?.querySelector<HTMLElement>('.page-content');
  const departingToolbar=toolbarExit;toolbarExit=null;
  if(!ready||!page||!location.startsWith('Library:')){origin=null;departingToolbar?.remove();return;}
  const motion=matchMedia('(prefers-reduced-motion: reduce)');
  if(motion.matches||document.documentElement.dataset.swipeNavigation){origin=null;departingToolbar?.remove();return;}
  const animations:Animation[]=[],disposers:(()=>void)[]=[];let frame=0,removed=false;
  const cancel=()=>{if(removed)return;removed=true;cancelAnimationFrame(frame);animations.forEach(animation=>animation.cancel());disposers.forEach(dispose=>dispose());};
  const changed=()=>{if(motion.matches)cancel();};motion.addEventListener('change',changed);
  const animate=(node:HTMLElement|null,delay:number,y=7,duration=180,id='album-detail-reveal',scale=1)=>{
   if(!node)return;disposers.push(animateOwned(node,id,[{opacity:0,transform:`translateY(${y}px) scale(${scale})`},{opacity:1,transform:'none'}],duration));
  };
  if(departingToolbar){
   page.closest('.content-column')?.append(departingToolbar);
   const exit=departingToolbar.animate([{opacity:1,transform:'translateY(0)'},{opacity:0,transform:'translateY(-8px)'}],{duration:140,easing:'ease-out',fill:'forwards'});
   exit.id='library-toolbar-exit';animations.push(exit);disposers.push(()=>departingToolbar.remove());
   void exit.finished.then(()=>departingToolbar.remove()).catch(()=>{});
  }else if(!/^Library:(album|artist):/.test(location)){
   animate(page.closest('.content-column')?.querySelector<HTMLElement>(':scope > .library-toolbar')??null,0,-6,180,'library-toolbar-enter');
  }
  const album=page.querySelector<HTMLElement>('.library-view:not([hidden]) .album-detail');
  const artist=page.querySelector<HTMLElement>('.library-view:not([hidden]) .artist-detail');
  if(album){
   const selectors=['.album-copy h1','.album-artist-credits','.album-footer'];
   selectors.forEach((selector,index)=>animate(album.querySelector(selector),0,5,180));
   const transferring=origin?.kind==='album'&&origin.id===album.dataset.albumId&&!origin.returning&&performance.now()-origin.at<750;
   if(!transferring)animate(album.querySelector('.album-artwork-stage'),0,0,180,'album-artwork-reveal');
   // A single row group keeps long albums cheap, with stable action columns.
   animate(album.querySelector('.track-table'),0,4,180);
  }else if(artist){
   artist.querySelectorAll<HTMLElement>('.artist-name-part').forEach((word,index)=>animate(word,Math.min(index*8,32),6,148,'artist-name-reveal'));
   const transferring=origin?.kind==='artist'&&origin.id===artist.dataset.artistId&&!origin.returning&&performance.now()-origin.at<750;
   if(!transferring)animate(artist.querySelector('.artist-portrait-button'),0,5,180,'artist-portrait-reveal',.92);
   ['.artist-about','.artist-summary','.album-strip'].forEach((selector,index)=>animate(artist.querySelector(selector),0,origin?.returning&&selector==='.album-strip'?0:5,180,'artist-detail-reveal'));
   animate(artist.querySelector('.track-table'),0,4,180,'artist-detail-reveal');
  }
  const captured=origin;origin=null;
  if(captured&&performance.now()-captured.at<750){
   // One final-bounds read, after native scroll restoration. No grid-wide layout pass.
   const destination=captured.kind==='artist'
    ?artist?.dataset.artistId===captured.id?artist.querySelector<HTMLElement>('.artist-portrait-button'):null
    :album?.dataset.albumId===captured.id?album.querySelector<HTMLElement>('.album-artwork-stage'):null;
   const waiting=!captured.returning?destination:null;
   const waitingOpacity=waiting?.style.opacity??'';
   if(waiting){waiting.style.opacity='0';disposers.push(()=>{waiting.style.opacity=waitingOpacity;});}
   let attempts=0;
   const transfer=()=>{
    if(removed)return;
    const target=captured.returning
     ?page.querySelector<HTMLElement>('.library-view:not([hidden])')?.querySelector<HTMLElement>(`[data-collection-id="${CSS.escape(captured.id)}"] .card-artwork${captured.kind==='album'?`,[data-album-id="${CSS.escape(captured.id)}"].artist-album-link > .cover`:''}`)
     :destination;
    if(!target){if(++attempts<3)frame=requestAnimationFrame(transfer);else if(waiting)waiting.style.opacity=waitingOpacity;return;}
    const end=target.getBoundingClientRect();if(!end.width||!end.height||end.bottom<=0||end.top>=innerHeight){if(waiting)waiting.style.opacity=waitingOpacity;return;}
    const image=document.createElement('img');image.className='album-cover-transfer';image.src=captured.url;image.alt='';image.setAttribute('aria-hidden','true');image.dataset.albumId=captured.id;image.dataset.coverKind=captured.kind;image.draggable=false;
    const clipping=target.querySelector<HTMLElement>('.album-artwork')??target;
    const curve=getComputedStyle(clipping).borderTopLeftRadius;
    const radius=curve.endsWith('%')?Math.min(end.width,end.height)*parseFloat(curve)/100:parseFloat(curve)||0;
    Object.assign(image.style,{left:`${end.left}px`,top:`${end.top}px`,width:`${end.width}px`,height:`${end.height}px`,clipPath:`path('${smoothPath(end.width,end.height,radius,Number(clipping.dataset.cornerSmoothing??(captured.kind==='artist'?.6:1)))}')`});
    const previous=target===waiting?waitingOpacity:target.style.opacity;target.style.opacity='0';target.dataset.albumEntering='true';document.body.append(image);
    let timeout:ReturnType<typeof setTimeout>|undefined,finished=false,movement:Animation|undefined;const initialScroll=root.current?.scrollTop??0;
    const scrolled=()=>{if(Math.abs((root.current?.scrollTop??0)-initialScroll)>1)release();};
    const resized=()=>release();
    const release=()=>{if(finished)return;finished=true;clearTimeout(timeout);movement?.cancel();image.remove();target.style.opacity=previous;delete target.dataset.albumEntering;target.removeEventListener('load',loaded,true);root.current?.removeEventListener('scroll',scrolled);window.removeEventListener('resize',resized);};
    const loaded=()=>{const photo=target.matches('img.cover:not(.placeholder)')?target as HTMLImageElement:target.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');if(photo?.complete&&photo.naturalWidth)release();};
    disposers.push(release);
    const animation=image.animate([
     {transform:`translate(${captured.rect.left-end.left}px,${captured.rect.top-end.top}px) scale(${captured.rect.width/end.width},${captured.rect.height/end.height})`},
     {transform:'translate(0,0) scale(1,1)'}
    ],{duration:320,easing:'cubic-bezier(.2,.75,.2,1)',fill:'backwards'});
    animation.id=`${captured.kind}-cover-transfer`;movement=animation;animations.push(animation);root.current?.addEventListener('scroll',scrolled,{passive:true});window.addEventListener('resize',resized);
    void animation.finished.then(()=>{if(removed)return;const photo=target.matches('img.cover:not(.placeholder)')?target as HTMLImageElement:target.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');if(photo?.complete&&photo.naturalWidth)release();else{target.addEventListener('load',loaded,true);timeout=setTimeout(release,1200);}}).catch(()=>{});
   };
   frame=requestAnimationFrame(transfer);
  }
  return()=>{cancel();motion.removeEventListener('change',changed);};
 },[root,location,ready]);
}
