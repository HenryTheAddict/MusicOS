import {smoothPath} from './smoothCorners';
/** The interactive gesture owns this artwork from pickup through commit/retract.
 * Measure once; each pointer frame only advances a paused native timeline. */
export function swipeCover(page:HTMLElement,preview:HTMLElement){
 const detail=page.querySelector<HTMLElement>('.album-detail[data-album-id],.artist-detail[data-artist-id]');if(!detail)return;
 const artist=!!detail.dataset.artistId,id=detail.dataset.artistId??detail.dataset.albumId!;
 const source=detail.querySelector<HTMLElement>(artist?'.artist-portrait-button':'.album-artwork-stage');
 const photo=source?.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');
 const targetSelector=`[data-collection-id="${CSS.escape(id)}"] .card-artwork,[data-album-id="${CSS.escape(id)}"].artist-album-link>.cover`;
 const target=preview.querySelector<HTMLElement>(targetSelector);
 if(!source||!photo?.complete||!photo.naturalWidth||!target)return;
 const moving=document.querySelector<HTMLImageElement>(`.album-cover-transfer[data-album-id="${CSS.escape(id)}"]`);
 const from=(moving??photo).getBoundingClientRect(),to=target.getBoundingClientRect(),viewport=page.parentElement!.getBoundingClientRect();
 if(!from.width||!to.width||to.bottom<=viewport.top||to.top>=viewport.bottom)return;
 const radius=(node:HTMLElement,size:number)=>{const raw=getComputedStyle(node).borderTopLeftRadius;return raw.endsWith('%')?size*parseFloat(raw)/100:parseFloat(raw)||0;};
 const sourceFrame=source.querySelector<HTMLElement>('.album-artwork')??source;
 const rounding=radius(sourceFrame,from.width),smoothing=artist ? .6 : 1,scale=to.width/from.width;
 const first={transform:'translate(0px,0px) scale(1)',clipPath:`path('${smoothPath(from.width,from.height,rounding,smoothing)}')`};
 const last={transform:`translate(${to.left-from.left}px,${to.top-from.top}px) scale(${scale})`,clipPath:`path('${smoothPath(from.width,from.height,radius(target,to.width)/scale,Number(target.dataset.cornerSmoothing??smoothing))}')`};
 const image=document.createElement('img');image.className='swipe-cover-transfer';image.src=photo.currentSrc||photo.src;image.alt='';image.ariaHidden='true';image.draggable=false;
 Object.assign(image.style,{left:`${from.left}px`,top:`${from.top}px`,width:`${from.width}px`,height:`${from.height}px`});window.dispatchEvent(new Event('musicos-back-gesture-start'));document.body.append(image);
 const oldSource=source.style.opacity,oldTarget=target.style.opacity;source.style.opacity='0';target.style.opacity='0';
 let timeline=image.animate([first,last],{duration:1000,fill:'both',easing:'linear'});timeline.id='interactive-back-cover';timeline.pause();timeline.currentTime=0;let disposed=false,manual=true;
 return {
  seek(progress:number){if(disposed)return;if(!manual){timeline.cancel();timeline=image.animate([first,last],{duration:1000,fill:'both',easing:'linear'});timeline.id='interactive-back-cover';timeline.pause();manual=true;}timeline.currentTime=Math.max(0,Math.min(1,progress))*1000;},
  finish(commit:boolean,duration:number){if(disposed)return Promise.resolve();manual=false;const style=getComputedStyle(image),shown={transform:style.transform,clipPath:style.clipPath};timeline.cancel();timeline=image.animate([shown,commit?last:first],{duration,fill:'both',easing:'cubic-bezier(.2,.7,.2,1)'});timeline.id='interactive-back-cover-finish';return timeline.finished.catch(()=>{});},
  destinationPaintable(){
   const live=page.querySelector<HTMLElement>(targetSelector);
   if(!live||!live.getClientRects().length)return false;
   const painted=live.matches('img.cover:not(.placeholder)')?live as HTMLImageElement:live.querySelector<HTMLImageElement>('img.cover:not(.placeholder)');
   return !!(painted?.complete&&painted.naturalWidth);
  },
  release(){if(disposed)return;disposed=true;timeline.cancel();image.remove();source.style.opacity=oldSource;target.style.opacity=oldTarget;},
 };
}
