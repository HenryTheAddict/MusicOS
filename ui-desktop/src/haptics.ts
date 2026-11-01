import {call,native} from './bridge';
import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import {beatOffsets} from './heartVisibility';
export type FeedbackSettings={enabled:boolean;beat_hover:boolean;interactions:boolean};
export type FeedbackKind='stop'|'beat'|'pickup'|'target'|'reorder'|'drop'|'favorite'|'toggle'|'step';
export const defaultFeedback:FeedbackSettings={enabled:/Mac/.test(navigator.platform),beat_hover:true,interactions:true};
let settings=defaultFeedback,hover:HTMLElement|null=null,timer=0,off:(()=>void)|undefined,lastBeat='',lastInteraction=-Infinity,beatSubmitted=false;
function cancelNativeBeat(){if(!beatSubmitted)return;beatSubmitted=false;void call('trackpad_feedback',{request:{kind:'stop',key:'hover-ended'}}).catch(()=>{});}
export function haptic(kind:FeedbackKind,key:string,trackId?:number){if(!native||!settings.enabled||document.hidden||!document.hasFocus()||(kind==='beat'?!settings.beat_hover:!settings.interactions))return;
 if(kind==='beat'&&performance.now()-lastInteraction<120)return;if(kind==='beat')beatSubmitted=true;if(kind!=='beat')lastInteraction=performance.now();void call('trackpad_feedback',{request:{kind,key,trackId}}).catch(()=>{});
}
export function setFeedbackSettings(value?:FeedbackSettings){settings=value??defaultFeedback;update();}
export function nextBeat(position:number,bpm:number,speed:number,offset:number){const every=Math.max(1,Math.ceil(bpm*speed/240)),beat=(position-offset)*bpm/60,next=Math.ceil((beat-1e-5)/every)*every;return{index:next,delay:Math.max(0,(next-beat)*60000/bpm/speed)};}
function update(){clearTimeout(timer);timer=0;const node=hover,c=currentPlaybackClock(),bpm=Number(node?.dataset.bpm),now=performance.now();if(!native||!settings.enabled||!settings.beat_hover||!node?.isConnected||node.dataset.pulsing!=='true'||node.closest('[inert],[hidden]')||document.hidden||!document.hasFocus()||!c.playing||c.trackId!==Number(node.dataset.heartTrack)||now-c.receivedAt>650||!Number.isFinite(bpm)||bpm<20||bpm>400||c.speed<=0){cancelNativeBeat();return;}
 const next=nextBeat(clockPosition(c),bpm,c.speed,beatOffsets.get(c.trackId!)??0),key=`${c.trackId}:${next.index}`;
 timer=window.setTimeout(()=>{timer=0;const fresh=currentPlaybackClock();if(hover!==node||node.dataset.pulsing!=='true'||node.closest('[inert],[hidden]')||fresh.trackId!==c.trackId||!fresh.playing||fresh.speed!==c.speed||performance.now()-fresh.receivedAt>650){update();return;}if(key!==lastBeat){lastBeat=key;haptic('beat',key,c.trackId);}timer=window.setTimeout(update,8);},Math.max(1,next.delay));
}
export function releaseHeart(node:HTMLElement){if(hover===node)hoverHeart(null);}
export function hoverHeart(node:HTMLElement|null){cancelNativeBeat();hover=node;clearTimeout(timer);timer=0;off?.();off=undefined;if(node){off=subscribePlaybackClock(update);update();}}
window.addEventListener('blur',()=>hoverHeart(null));document.addEventListener('visibilitychange',()=>{if(document.hidden)hoverHeart(null);});
/** Only committed discrete controls. The seek rail is deliberately excluded. */
export function installInteractionFeedback(){
 const range=(node:EventTarget|null)=>node instanceof Element?node.closest<HTMLElement>('input[type=range],[role=slider]'):null;
 let slider:HTMLElement|null=null,before='';const value=(n:HTMLElement)=>n instanceof HTMLInputElement?n.value:n.getAttribute('aria-valuenow')??'';
 const begin=(e:Event)=>{const n=range(e.target);if(n&&!n.closest('.playback-progress,.seek,.progress,.expanded-player-progress')){slider=n;before=value(n);}};
 const cancel=()=>{slider=null;};
 const commit=()=>{if(slider&&value(slider)!==before)haptic('step',`${slider.getAttribute('aria-label')}:${value(slider)}`);slider=null;};
 const keyup=(e:KeyboardEvent)=>{if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown'].includes(e.key))commit();};
 const click=(e:MouseEvent)=>{const n=e.target instanceof Element?e.target.closest<HTMLElement>('button[aria-pressed]'):null;if(n&&!n.classList.contains('track-favorite'))haptic('toggle',n.getAttribute('aria-label')??n.textContent??'toggle');};
 document.addEventListener('pointerdown',begin,true);document.addEventListener('keydown',begin,true);document.addEventListener('pointerup',commit,true);document.addEventListener('pointercancel',cancel,true);document.addEventListener('keyup',keyup,true);document.addEventListener('click',click,true);
 return()=>{document.removeEventListener('pointerdown',begin,true);document.removeEventListener('keydown',begin,true);document.removeEventListener('pointerup',commit,true);document.removeEventListener('pointercancel',cancel,true);document.removeEventListener('keyup',keyup,true);document.removeEventListener('click',click,true);hoverHeart(null);};
}
