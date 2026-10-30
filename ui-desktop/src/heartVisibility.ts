import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
export const beatOffsets=new Map<number,number>();
const hearts=new Map<HTMLElement,boolean>();
let observer:IntersectionObserver|undefined,off:(()=>void)|undefined;
function updateVisibility(){for(const [heart,visible] of hearts){const on=visible&&!document.hidden;if(heart.dataset.pulsing!==String(on))heart.dataset.pulsing=String(on);if(!on)continue;const clock=currentPlaybackClock(),id=Number(heart.dataset.heartTrack),bpm=Number(heart.dataset.bpm);if(clock.trackId!==id||!clock.playing||!Number.isFinite(bpm)||bpm<20)continue;const animation=heart.querySelector('.heart-beat')?.getAnimations().find(a=>a instanceof CSSAnimation);if(!animation)continue;const period=60000/bpm,offset=beatOffsets.get(id)??0,target=((clockPosition(clock)*1000-offset*1000+period*.1)%period+period)%period;const actual=Number(animation.currentTime??0)%period,drift=Math.abs(target-actual);if(Math.min(drift,period-drift)>24)animation.currentTime=target;if(animation.playbackRate!==clock.speed)animation.playbackRate=clock.speed;}}
export function watchHeart(heart:HTMLElement){
 if(typeof IntersectionObserver==='undefined'){heart.dataset.pulsing='false';return()=>{delete heart.dataset.pulsing;};}
 if(!observer){observer=new IntersectionObserver(entries=>{for(const entry of entries)if(hearts.has(entry.target as HTMLElement))hearts.set(entry.target as HTMLElement,entry.isIntersecting);updateVisibility();});document.addEventListener('visibilitychange',updateVisibility);off=subscribePlaybackClock(updateVisibility);}
 hearts.set(heart,false);observer.observe(heart);
 return()=>{observer?.unobserve(heart);hearts.delete(heart);delete heart.dataset.pulsing;if(!hearts.size){observer?.disconnect();observer=undefined;off?.();off=undefined;document.removeEventListener('visibilitychange',updateVisibility);}};
}
