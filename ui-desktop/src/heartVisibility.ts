import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
export const beatOffsets=new Map<number,number>();
const hearts=new Map<HTMLElement,boolean>();
const phases=new WeakMap<HTMLElement,{animation:Animation;position:number;receivedAt:number;speed:number}>();
let observer:IntersectionObserver|undefined,off:(()=>void)|undefined;
function updateVisibility(){for(const [heart,visible] of hearts){const on=visible&&!document.hidden;if(heart.dataset.pulsing!==String(on))heart.dataset.pulsing=String(on);if(!on)continue;const clock=currentPlaybackClock(),id=Number(heart.dataset.heartTrack),bpm=Number(heart.dataset.bpm);if(clock.trackId!==id||!clock.playing||!Number.isFinite(bpm)||bpm<20)continue;const animation=(heart.matches('.cover-beat')?heart:heart.querySelector('.heart-beat'))?.getAnimations().find(a=>a instanceof CSSAnimation);if(!animation)continue;const period=60000/bpm,offset=beatOffsets.get(id)??0,target=((clockPosition(clock)*1000-offset*1000+period*.1)%period+period)%period;const actual=Number(animation.currentTime??0)%period,drift=Math.abs(target-actual);const prior=phases.get(heart),jump=prior&&Math.abs(clock.position-(prior.position+(clock.receivedAt-prior.receivedAt)/1000*prior.speed))>.15;if(!prior||prior.animation!==animation||jump||Math.min(drift,period-drift)>period*.4)animation.currentTime=target;phases.set(heart,{animation,position:clock.position,receivedAt:clock.receivedAt,speed:clock.speed});if(animation.playbackRate!==clock.speed)animation.playbackRate=clock.speed;}}
function resetPhase(){for(const heart of hearts.keys())phases.delete(heart);updateVisibility();}
export function watchHeart(heart:HTMLElement){
 if(typeof IntersectionObserver==='undefined'){heart.dataset.pulsing='false';return()=>{delete heart.dataset.pulsing;};}
 if(!observer){observer=new IntersectionObserver(entries=>{for(const entry of entries)if(hearts.has(entry.target as HTMLElement))hearts.set(entry.target as HTMLElement,entry.isIntersecting);updateVisibility();});document.addEventListener('visibilitychange',updateVisibility);window.addEventListener('musicos-playback-discontinuity',resetPhase);off=subscribePlaybackClock(updateVisibility);}
 hearts.set(heart,false);observer.observe(heart);
 return()=>{observer?.unobserve(heart);hearts.delete(heart);delete heart.dataset.pulsing;if(!hearts.size){observer?.disconnect();observer=undefined;off?.();off=undefined;document.removeEventListener('visibilitychange',updateVisibility);window.removeEventListener('musicos-playback-discontinuity',resetPhase);}};
}
