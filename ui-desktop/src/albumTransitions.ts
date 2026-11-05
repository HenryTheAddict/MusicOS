import {animateOwned} from './presentationMotion';
import {useLayoutEffect,type RefObject} from 'react';
import {smoothPath} from './smoothCorners';

type CoverKind='album'|'artist';
type CoverOrigin={kind:CoverKind;id:string;url:string;rect:DOMRect;returning:boolean;at:number;radius:number;smoothing:number};
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
 if(moving?.dataset.coverKind===kind&&moving.dataset.albumId===id){origin={kind,id,url:moving.src,rect:moving.getBoundingClientRect(),returning,at:performance.now(),radius:parseFloat(getComputedStyle(moving).borderTopLeftRadius)||0,smoothing:Number(moving.dataset.cornerSmoothing??1)};return;}
 const image=surface.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');
 if(!image?.complete||!image.naturalWidth)return;
 const rect=image.getBoundingClientRect();
 if(!rect.width||!rect.height||rect.bottom<=0||rect.top>=innerHeight)return;
 const frame=image.closest<HTMLElement>('.card-artwork,.album-artwork,.artist-portrait-button')??image;const raw=getComputedStyle(frame).borderTopLeftRadius;
 origin={kind,id,url:image.currentSrc||image.src,rect,returning,at:performance.now(),radius:raw.endsWith('%')?rect.width*parseFloat(raw)/100:parseFloat(raw)||0,smoothing:Number(frame.dataset.cornerSmoothing??(kind==='artist'?.6:1))};
}
export function coverTransferPending(){return !!origin&&performance.now()-origin.at<750;}
export function captureAlbumCover(id:string,surface?:HTMLElement){snapshot('album',id,surface,false);}
export function captureAlbumReturn(root:HTMLElement|null){const album=root?.querySelector<HTMLElement>('.album-detail[data-album-id]');snapshot('album',album?.dataset.albumId??'',album,true);}

export function captureArtistCover(id:string,surface?:HTMLElement){snapshot('artist',id,surface,false);}
export function captureArtistReturn(root:HTMLElement|null){const artist=root?.querySelector<HTMLElement>('.artist-detail[data-artist-id]');snapshot('artist',artist?.dataset.artistId??'',artist,true);}

/** One artwork owner, measured against the settled layout. Copy never moves its anchor. */
export function useAlbumReveal(root:RefObject<HTMLElement|null>,location:string,ready:boolean){
 useLayoutEffect(()=>{
  const page=root.current?.querySelector<HTMLElement>('.page-content'),departingToolbar=toolbarExit,captured=origin;
  toolbarExit=null;origin=null;
  if(!ready||!page||!location.startsWith('Library:')){departingToolbar?.remove();return;}
  const motion=matchMedia('(prefers-reduced-motion: reduce)');
  if(motion.matches||document.documentElement.dataset.swipeNavigation){departingToolbar?.remove();return;}
  const disposers:(()=>void)[]=[];let frame=0,removed=false;
  const cancel=()=>{if(removed)return;removed=true;cancelAnimationFrame(frame);disposers.forEach(dispose=>dispose());};
  const changed=()=>{if(motion.matches)cancel();};motion.addEventListener('change',changed);
  const returning=!!captured?.returning;
  const reveal=(node:HTMLElement|null,id='album-detail-reveal',spatial=true)=>{if(node)disposers.push(animateOwned(node,id,[{opacity:0,transform:spatial?`translateX(${returning?-6:6}px)`:'none'},{opacity:1,transform:'none'}],180));};
  if(departingToolbar){
   page.closest('.content-column')?.append(departingToolbar);
   disposers.push(animateOwned(departingToolbar,'library-toolbar-exit',[{opacity:1},{opacity:0}],140),()=>departingToolbar.remove());
   const timer=setTimeout(()=>departingToolbar.remove(),140);disposers.push(()=>clearTimeout(timer));
  }
  const album=page.querySelector<HTMLElement>('.library-view:not([hidden]) .album-detail');
  const artist=page.querySelector<HTMLElement>('.library-view:not([hidden]) .artist-detail');
  if(album){
   ['.album-copy h1','.album-artist-credits','.album-footer','.track-table'].forEach(selector=>reveal(album.querySelector(selector)));
  }else if(artist){
   reveal(artist.querySelector('.artist-hero>h1'),'artist-name-reveal');
   ['.artist-about','.artist-statistics','.artist-playback','.album-strip','.track-table'].forEach(selector=>reveal(artist.querySelector(selector),'artist-detail-reveal',selector!=='.album-strip'));
  }
  if(captured&&performance.now()-captured.at<750){
   const destination=()=>captured.returning
    ?page.querySelector<HTMLElement>('.library-view:not([hidden])')?.querySelector<HTMLElement>(`[data-collection-id="${CSS.escape(captured.id)}"] .card-artwork${captured.kind==='album'?`,[data-album-id="${CSS.escape(captured.id)}"].artist-album-link > .cover`:''}`)??null
    :captured.kind==='artist'
      ?artist?.dataset.artistId===captured.id?artist.querySelector<HTMLElement>('.artist-portrait-button'):null
      :album?.dataset.albumId===captured.id?album.querySelector<HTMLElement>('.album-artwork-stage'):null;
   const image=document.createElement('img');image.className='album-cover-transfer';image.src=captured.url;image.alt='';image.setAttribute('aria-hidden','true');image.dataset.albumId=captured.id;image.dataset.coverKind=captured.kind;image.dataset.cornerSmoothing=String(captured.smoothing);image.draggable=false;
   Object.assign(image.style,{left:`${captured.rect.left}px`,top:`${captured.rect.top}px`,width:`${captured.rect.width}px`,height:`${captured.rect.height}px`,borderRadius:`${captured.radius}px`,clipPath:`path('${smoothPath(captured.rect.width,captured.rect.height,captured.radius,captured.smoothing)}')`});
   // Keep the source pixels visible across the route commit, not a blank frame.
   document.body.append(image);
   let target:HTMLElement|null=null,previous='',movement:Animation|undefined,resize:ResizeObserver|undefined,timeout:ReturnType<typeof setTimeout>|undefined,finished=false,attempts=0,deadline=0,lastEnd:DOMRect|undefined,initialScroll=0;
   const hide=(node:HTMLElement)=>{target=node;previous=node.style.opacity;node.style.opacity='0';node.dataset.albumEntering='true';};
   const restore=()=>{if(target){target.style.opacity=previous;delete target.dataset.albumEntering;}};
   const release=()=>{if(finished)return;finished=true;cancelAnimationFrame(frame);clearTimeout(timeout);resize?.disconnect();movement?.cancel();restore();image.remove();target?.removeEventListener('load',loaded,true);root.current?.removeEventListener('scroll',scrolled);window.removeEventListener('resize',release);};
   const photo=()=>target?.matches('img.cover:not(.placeholder)')?target as HTMLImageElement:target?.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');
   const loaded=()=>{const p=photo();if(p?.complete&&p.naturalWidth)release();};
   const scrolled=()=>{if(Math.abs((root.current?.scrollTop??0)-initialScroll)>1)release();};
   const place=(end:DOMRect,from:DOMRect,duration:number)=>{
    const clipping=target?.querySelector<HTMLElement>('.album-artwork')??target!,raw=getComputedStyle(clipping).borderTopLeftRadius;
    const radius=raw.endsWith('%')?Math.min(end.width,end.height)*parseFloat(raw)/100:parseFloat(raw)||0,smoothing=Number(clipping.dataset.cornerSmoothing??(captured.kind==='artist'?.6:1));
    movement?.cancel();lastEnd=end;
    Object.assign(image.style,{left:`${end.left}px`,top:`${end.top}px`,width:`${end.width}px`,height:`${end.height}px`,borderRadius:`${radius}px`});
    const startPath=smoothPath(end.width,end.height,captured.radius/Math.max(1,captured.rect.width)*end.width,captured.smoothing),endPath=smoothPath(end.width,end.height,radius,smoothing);
    const animation=image.animate([{transform:`translate(${from.left-end.left}px,${from.top-end.top}px) scale(${from.width/end.width},${from.height/end.height})`,clipPath:`path('${startPath}')`},{transform:'translate(0,0) scale(1,1)',clipPath:`path('${endPath}')`}],{duration,easing:'cubic-bezier(.22,.8,.24,1)',fill:'both'});
    animation.id=`${captured.kind}-cover-transfer`;movement=animation;
    void animation.finished.then(()=>{if(finished||movement!==animation)return;const p=photo();if(p?.complete&&p.naturalWidth)release();else{target?.addEventListener('load',loaded,true);timeout=setTimeout(release,1200);}}).catch(()=>{});
   };
   const retarget=()=>{if(finished||!target||!lastEnd)return;const end=target.getBoundingClientRect();if(!end.width||!end.height)return;if(Math.max(Math.abs(end.x-lastEnd.x),Math.abs(end.y-lastEnd.y),Math.abs(end.width-lastEnd.width),Math.abs(end.height-lastEnd.height))<.5)return;const shown=image.getBoundingClientRect();place(end,shown,Math.max(50,deadline-performance.now()));};
   const transfer=()=>{
    frame=0;if(removed||finished)return;const next=destination();
    if(!next){if(++attempts<4)frame=requestAnimationFrame(transfer);else release();return;}
    if(!target)hide(next);
    const end=next.getBoundingClientRect();if(!end.width||!end.height||end.bottom<=0||end.top>=innerHeight){release();return;}
    initialScroll=root.current?.scrollTop??0;deadline=performance.now()+320;
    place(end,captured.rect,320);
    resize=new ResizeObserver(retarget);resize.observe(next);if(next.parentElement)resize.observe(next.parentElement);
    root.current?.addEventListener('scroll',scrolled,{passive:true});window.addEventListener('resize',release);
   };
   const immediate=destination();if(immediate)hide(immediate);
   disposers.push(release);frame=requestAnimationFrame(transfer);
  }
  return()=>{cancel();motion.removeEventListener('change',changed);};
 },[root,location,ready]);
}
