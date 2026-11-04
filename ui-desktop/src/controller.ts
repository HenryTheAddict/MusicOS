import {call,native,subscribe} from './bridge';
import type {FeedbackKind} from './haptics';
import {nextBeat,setControllerFeedbackSink} from './haptics';
import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import {beatOffsets} from './heartVisibility';
export type ControllerPreferences={enabled:boolean;vibration:boolean;beat_focus:boolean};
export const defaultController:ControllerPreferences={enabled:true,vibration:true,beat_focus:true};
export type ControllerStatus={devices:{id:number;name:string;mapped:boolean;vibration:boolean}[];active:number|null;error:string|null};
export type ControllerInput={device:number;control:string;value:number};
export type ControllerRequest={action:'configure';active:boolean;sequence:number}|{action:'status'|'stop'}|{action:'feedback';kind:FeedbackKind;key:string;trackId?:number};
export type Direction='up'|'down'|'left'|'right';
export type ControllerActions={play:()=>void;previous:()=>void;next:()=>void;panel:()=>void;visualizer:()=>void;favorite:()=>void;seek:(delta:number)=>void;back:()=>void;error?:(e:unknown)=>void};
export const emptyController:ControllerStatus={devices:[],active:null,error:null};
let sequence=0;
const configureRequest=(active:boolean):ControllerRequest=>({action:'configure',active,sequence:sequence=Math.max(sequence+1,Math.floor((performance.timeOrigin+performance.now())*1000))});
const directions=new Set(['up','down','left','right']);
export function axisDirection(value:number,prior:number){if(!Number.isFinite(value)||Math.abs(value)<.35)return 0;return Math.abs(value)>=.55?Math.sign(value):Math.sign(value)===prior?prior:0;}
export function navigationScore(from:{x:number;y:number},to:{x:number;y:number},direction:Direction){const dx=to.x-from.x,dy=to.y-from.y,forward=direction==='left'?-dx:direction==='right'?dx:direction==='up'?-dy:dy;return forward>2?forward+Math.abs(direction==='up'||direction==='down'?dx:dy)*2.5:Infinity;}
const visible=(el:HTMLElement)=>!el.closest('[hidden],[inert],[aria-hidden=true]')&&el.getClientRects().length>0&&getComputedStyle(el).visibility!=='hidden'&&getComputedStyle(el).opacity!=='0';
export function controllerScope(){const shown=<T extends HTMLElement>(selector:string)=>Array.from(document.querySelectorAll<T>(selector)).filter(visible).at(-1);return shown<HTMLElement>('[role=menu]')??shown<HTMLDialogElement>('dialog[open]')??shown<HTMLElement>('.mini-player')??shown<HTMLElement>('.app-shell')??document.body;}
function controls(scope:HTMLElement){return Array.from(scope.querySelectorAll<HTMLElement>('button,summary,a[href],input:not([type=hidden]),select,textarea,[tabindex],[role=menuitem],[role=menuitemradio],[role=menuitemcheckbox]')).filter(el=>visible(el)&&!el.matches(':disabled,[aria-disabled=true]')&&el.tabIndex>=0);}
function eventKey(target:HTMLElement,key:string){const down=new KeyboardEvent('keydown',{key,code:key,bubbles:true,cancelable:true});target.dispatchEvent(down);target.dispatchEvent(new KeyboardEvent('keyup',{key,code:key,bubbles:true,cancelable:true}));return down.defaultPrevented;}
/** Radix popup triggers open on Enter/pointer-down, not a synthetic click. */
export function activateControl(target:HTMLElement){if(target.matches('button[aria-haspopup],[role=combobox]')&&eventKey(target,'Enter'))return;target.click();}
export function focusDirection(direction:Direction){const scope=controllerScope(),active=document.activeElement as HTMLElement|null;
 if(active&&scope.contains(active)&&active.matches('input[type=range],[role=slider]')&&(direction==='left'||direction==='right')){if(active instanceof HTMLInputElement){const before=active.value;direction==='left'?active.stepDown():active.stepUp();if(active.value!==before){active.dispatchEvent(new Event('input',{bubbles:true}));active.dispatchEvent(new Event('change',{bubbles:true}));}}else eventKey(active,direction==='left'?'ArrowLeft':'ArrowRight');return true;}
 if(active&&scope.contains(active)&&active.matches('[role=tab]')){const tabs=active.closest('[role=tablist]'),vertical=tabs?.getAttribute('aria-orientation')==='vertical';if(tabs&&((vertical&&(direction==='up'||direction==='down'))||(!vertical&&(direction==='left'||direction==='right')))){if(eventKey(active,{up:'ArrowUp',down:'ArrowDown',left:'ArrowLeft',right:'ArrowRight'}[direction]))return true;}}
 if(scope.matches('[role=menu]')){eventKey(active&&scope.contains(active)?active:scope,{up:'ArrowUp',down:'ArrowDown',left:'ArrowLeft',right:'ArrowRight'}[direction]);return true;}
 if(active instanceof HTMLSelectElement&&scope.contains(active)&&(direction==='left'||direction==='right')){const options=Array.from(active.options).filter(o=>!o.disabled),at=options.findIndex(o=>o.selected),next=options[Math.max(0,Math.min(options.length-1,at+(direction==='left'?-1:1)))];if(next&&next.value!==active.value){active.value=next.value;active.dispatchEvent(new Event('input',{bubbles:true}));active.dispatchEvent(new Event('change',{bubbles:true}));}return true;}
 const candidates=controls(scope);if(!candidates.length)return false;let next:HTMLElement|undefined;
 if(!active||!scope.contains(active)||!visible(active)||active===document.body)next=candidates[0];else{const a=active.getBoundingClientRect(),from={x:a.x+a.width/2,y:a.y+a.height/2};let best=Infinity;for(const el of candidates){if(el===active)continue;const b=el.getBoundingClientRect(),score=navigationScore(from,{x:b.x+b.width/2,y:b.y+b.height/2},direction);if(score<best){next=el;best=score;}}}
 if(!next)return false;next.focus({preventScroll:true});next.scrollIntoView({block:'nearest',inline:'nearest',behavior:'instant'});return true;
}
export function controllerBack(fallback:()=>void){const scope=controllerScope(),focused=document.activeElement as HTMLElement|null;if(scope.matches('[role=menu]')){eventKey(focused&&scope.contains(focused)?focused:scope,'Escape');return;}if(scope instanceof HTMLDialogElement){const cancel=new Event('cancel',{cancelable:true});if(scope.dispatchEvent(cancel))scope.close();return;}if(focused?.matches('input,textarea,select,[contenteditable=true]')){focused.blur();return;}if(eventKey(focused&&scope.contains(focused)?focused:scope,'Escape'))return;fallback();}
function scrollView(value:number){const scope=controllerScope();let el=document.activeElement instanceof HTMLElement&&scope.contains(document.activeElement)?document.activeElement:scope;const scrollable=(n:HTMLElement)=>n.clientHeight>0&&n.scrollHeight>n.clientHeight+1&&/auto|scroll/.test(getComputedStyle(n).overflowY);while(el!==scope&&!scrollable(el))el=el.parentElement??scope;if(!scrollable(el))el=Array.from(scope.querySelectorAll<HTMLElement>('*')).filter(n=>visible(n)&&scrollable(n)).sort((a,b)=>b.clientHeight-a.clientHeight)[0]??scope;el.scrollBy({top:-value*48,left:0,behavior:'instant'});}
/** Native events have one focused-window owner. Only held navigation/scrolling
 * repeat; transport, triggers and activation are rising-edge actions. */
export function installController(prefs:ControllerPreferences,actions:ControllerActions){
 if(!native)return()=>{};
 let alive=true,timer=0,beatTimer=0,beatSubmitted=false,axisX=0,axisY=0,scrollY=0,repeat:Direction|null=null,lastBeat='',lastPulse=-Infinity;
 const held=new Set<string>(),off:(()=>void)[]=[];let lastDevice:number|null=null;
 const usable=()=>prefs.enabled&&!document.hidden&&document.hasFocus();
 const send=(request:ControllerRequest)=>{if(alive)void call('controller',{request}).catch(actions.error??(()=>{}));};
 const rumble=(kind:FeedbackKind,key:string,trackId?:number)=>{if(usable()&&prefs.vibration&&lastDevice!==null){if(kind==='beat')beatSubmitted=true;send({action:'feedback',kind,key,trackId});}};
 const stopBeat=()=>{clearTimeout(beatTimer);beatTimer=0;if(beatSubmitted){beatSubmitted=false;send({action:'feedback',kind:'stop',key:'focus-ended'});}};
 const syncBeat=()=>{clearTimeout(beatTimer);beatTimer=0;const node=document.activeElement instanceof HTMLElement?document.activeElement.closest<HTMLElement>('.track-favorite'):null,c=currentPlaybackClock(),bpm=Number(node?.dataset.bpm),now=performance.now();
  if(!usable()||!prefs.vibration||!prefs.beat_focus||lastDevice===null||!node||!visible(node)||node.dataset.pulsing!=='true'||!c.playing||c.trackId!==Number(node.dataset.heartTrack)||now-c.receivedAt>650||!Number.isFinite(bpm)||bpm<20||bpm>400||c.speed<=0){stopBeat();return;}
  const next=nextBeat(clockPosition(c),bpm,c.speed,beatOffsets.get(c.trackId!)??0),key=`${c.trackId}:${next.index}`;
  beatTimer=window.setTimeout(()=>{beatTimer=0;const fresh=currentPlaybackClock();if(!usable()||document.activeElement!==node||fresh.trackId!==c.trackId||!fresh.playing||fresh.speed!==c.speed||performance.now()-fresh.receivedAt>650){syncBeat();return;}if(key!==lastBeat){lastBeat=key;rumble('beat',key,c.trackId);}beatTimer=window.setTimeout(syncBeat,8);},Math.max(1,next.delay));
 };
 const tick=()=>{timer=0;if(!usable())return;if(repeat){if(focusDirection(repeat))rumble('step',`focus:${document.activeElement?.getAttribute('aria-label')??document.activeElement?.textContent?.slice(0,60)}`);}if(Math.abs(scrollY)>.22)scrollView(scrollY);if(repeat||Math.abs(scrollY)>.22)timer=window.setTimeout(tick,repeat?125:70);};
 const beginRepeat=(value:Direction|null)=>{if(repeat===value)return;repeat=value;clearTimeout(timer);timer=0;if(value){if(focusDirection(value))rumble('step',`focus:${value}`);timer=window.setTimeout(tick,350);}else if(Math.abs(scrollY)>.22)timer=window.setTimeout(tick,70);};
 const reset=()=>{lastDevice=null;held.clear();axisX=axisY=scrollY=0;repeat=null;clearTimeout(timer);timer=0;stopBeat();send({action:'stop'});delete document.documentElement.dataset.controllerInput;};
 const edge=(input:ControllerInput)=>{if(input.control==='reset'){reset();lastDevice=null;return;}if(!usable())return;lastDevice=input.device;document.documentElement.dataset.controllerInput='true';if(Math.abs(input.value)>.22)window.dispatchEvent(new Event('musicos-player-interaction'));
  if(input.control==='focus_x'||input.control==='focus_y'){if(input.control==='focus_x')axisX=axisDirection(input.value,axisX);else axisY=axisDirection(input.value,axisY);beginRepeat(axisY?(axisY>0?'up':'down'):axisX?(axisX>0?'right':'left'):null);return;}
  if(input.control==='scroll_y'){scrollY=Number.isFinite(input.value)?input.value:0;if(!timer&&Math.abs(scrollY)>.22)tick();return;}
  if(input.value<.5){held.delete(input.control);if(directions.has(input.control)&&repeat===input.control)beginRepeat(null);return;}if(held.has(input.control))return;held.add(input.control);
  if(directions.has(input.control)){beginRepeat(input.control as Direction);return;}
  if(input.control==='accept'){const scope=controllerScope(),target=document.activeElement;if(target instanceof HTMLElement&&scope.contains(target)&&visible(target)&&!target.matches(':disabled,[aria-disabled=true]')){activateControl(target);rumble('toggle','activate');}else focusDirection('down');}
  else if(input.control==='back')controllerBack(actions.back);
  else if(input.control==='context'){const target=document.activeElement;if(target instanceof HTMLElement&&controllerScope().contains(target)){const r=target.getBoundingClientRect();target.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:r.x+r.width/2,clientY:r.y+r.height/2,button:2}));}}
  else if(input.control==='play')actions.play();else if(input.control==='previous')actions.previous();else if(input.control==='next')actions.next();else if(input.control==='panel')actions.panel();else if(input.control==='visualizer')actions.visualizer();else if(input.control==='favorite')actions.favorite();else if(input.control==='seek_back')actions.seek(-10);else if(input.control==='seek_forward')actions.seek(10);
  if(input.control!=='accept'&&input.control!=='favorite'&&performance.now()-lastPulse>65){lastPulse=performance.now();rumble('toggle',`control:${input.control}`);}
  syncBeat();
 };
 const focus=()=>{send(configureRequest(usable()));if(!usable())reset();else syncBeat();};
 const pointer=()=>{delete document.documentElement.dataset.controllerInput;stopBeat();};
 window.addEventListener('focus',focus);window.addEventListener('blur',focus);document.addEventListener('visibilitychange',focus);document.addEventListener('focusin',syncBeat);document.addEventListener('focusout',stopBeat);document.addEventListener('pointerdown',pointer,true);
 setControllerFeedbackSink((kind,key,track)=>rumble(kind,key,track));off.push(subscribePlaybackClock(syncBeat));void subscribe<ControllerInput>('musicos-controller-input',edge).then(f=>{if(alive)off.push(f);else f();}).catch(actions.error??(()=>{}));focus();
 return()=>{reset();alive=false;off.forEach(f=>f());setControllerFeedbackSink(undefined);window.removeEventListener('focus',focus);window.removeEventListener('blur',focus);document.removeEventListener('visibilitychange',focus);document.removeEventListener('focusin',syncBeat);document.removeEventListener('focusout',stopBeat);document.removeEventListener('pointerdown',pointer,true);void call('controller',{request:configureRequest(false)}).catch(()=>{});};
}
