import {useEffect,useRef} from 'react';
import {call,errorText} from './bridge';
export function miniDimensions(width:number,height:number,density?:"standard"|"compact"|"wide"|boolean){return {width:Math.max(density==='wide'?360:density==='compact'?220:244,Math.round(width)),height:Math.max(density==='wide'?144:density==='compact'?136:320,Math.round(height))};}
export default function MiniResize({onError,density="standard"}:{density?:"standard"|"compact"|"wide";onError:(message:string)=>void}){
 const latest=useRef<{width:number;height:number}|null>(null),running=useRef(false),alive=useRef(true);
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;latest.current=null;};},[]);
 async function request(width:number,height:number){latest.current=miniDimensions(width,height,density);if(running.current)return;running.current=true;try{while(latest.current&&alive.current){const size=latest.current;latest.current=null;await call('mini_resize',size);}}catch(error){latest.current=null;if(alive.current)onError(`Could not resize Mini Player. ${errorText(error)}`);}finally{running.current=false;}}
 return <>{(['e','s','se']).map(edge=><div key={edge} className="mini-resize-edge" data-edge={edge} role="separator" aria-orientation={edge==='s'?'horizontal':'vertical'} aria-label={`Resize Mini Player ${edge==='e'?'width':edge==='s'?'height':'width and height'}`} tabIndex={0} onMouseDown={e=>e.stopPropagation()} onPointerDown={e=>{
  if(e.button!==0)return;const root=e.currentTarget.closest<HTMLElement>('.mini-player');if(!root||root.style.getPropertyValue('--mini-player-height'))return;e.preventDefault();e.stopPropagation();const node=e.currentTarget,rect=root.getBoundingClientRect(),x=e.screenX,y=e.screenY;node.setPointerCapture(e.pointerId);
  const move=(event:PointerEvent)=>void request(rect.width+(edge.includes('e')?event.screenX-x:0),rect.height+(edge.includes('s')?event.screenY-y:0));
  const stop=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('lostpointercapture',stop);};node.addEventListener('pointermove',move);node.addEventListener('lostpointercapture',stop);
 }} onKeyDown={e=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key))return;e.preventDefault();e.stopPropagation();const rect=e.currentTarget.closest('.mini-player')!.getBoundingClientRect(),step=e.shiftKey?40:10;void request(rect.width+(edge.includes('e')?(e.key==='ArrowRight'?step:e.key==='ArrowLeft'?-step:0):0),rect.height+(edge.includes('s')?(e.key==='ArrowDown'?step:e.key==='ArrowUp'?-step:0):0));}}/>)}</>;
}
