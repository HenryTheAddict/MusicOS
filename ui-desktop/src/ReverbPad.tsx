import {useId,useRef,type KeyboardEvent,type PointerEvent,type CSSProperties} from 'react';
const clamp=(n:number)=>Math.round(Math.max(0,Math.min(1,n))*100)/100;
/** Two real listening parameters, with pointer capture and independent accessible axes. */
export default function ReverbPad({wet,depth,onChange}:{wet:number;depth:number;onChange:(wet:number,depth:number)=>void}){
 const id=useId(),drag=useRef<{pointer:number;wet:number;depth:number}|null>(null);
 function move(e:PointerEvent<HTMLDivElement>){const box=e.currentTarget.getBoundingClientRect(),inset=10;onChange(clamp((e.clientX-box.left-inset)/(box.width-2*inset)),clamp(1-(e.clientY-box.top-inset)/(box.height-2*inset)));}
 function key(e:KeyboardEvent<HTMLDivElement>){if(e.target!==e.currentTarget)return;const step=e.shiftKey?.1:.01;let x=wet,y=depth;switch(e.key){case 'ArrowLeft':x-=step;break;case 'ArrowRight':x+=step;break;case 'ArrowUp':y+=step;break;case 'ArrowDown':y-=step;break;case 'Home':x=0;break;case 'End':x=1;break;case 'Escape':if(!drag.current)return;({wet:x,depth:y}=drag.current);e.currentTarget.releasePointerCapture(drag.current.pointer);drag.current=null;e.stopPropagation();break;default:return;}e.preventDefault();onChange(clamp(x),clamp(y));}
 return <div className="reverb-pad-wrap"><div className="reverb-pad" role="group" tabIndex={0} aria-label={`Reverb pad, ${Math.round(wet*100)}% room, ${Math.round(depth*100)}% depth`} aria-describedby={id} title="Left/right: room amount · Up/down: depth" style={{'--wet':wet,'--depth':depth} as CSSProperties} onKeyDown={key}
 onPointerDown={e=>{if(e.button!==0)return;e.preventDefault();e.currentTarget.focus({preventScroll:true});drag.current={pointer:e.pointerId,wet,depth};e.currentTarget.setPointerCapture(e.pointerId);move(e);}}
 onPointerMove={e=>{if(drag.current?.pointer===e.pointerId)move(e);}}
 onPointerUp={e=>{if(drag.current?.pointer===e.pointerId){e.currentTarget.releasePointerCapture(e.pointerId);drag.current=null;}}}
 onPointerCancel={()=>{drag.current=null;}} onLostPointerCapture={()=>{drag.current=null;}}>
 <span className="reverb-pad-cross" aria-hidden="true"/><span className="reverb-pad-handle" aria-hidden="true"/>
 <label className="reverb-axis">Room<input tabIndex={-1} aria-label="Reverb amount" type="range" min="0" max="1" step=".01" value={wet} onChange={e=>onChange(Number(e.target.value),depth)}/></label>
 <label className="reverb-axis">Depth<input tabIndex={-1} aria-label="Reverb depth" type="range" min="0" max="1" step=".01" value={depth} onChange={e=>onChange(wet,Number(e.target.value))}/></label>
 </div><span id={id} className="reverb-axis">Left and right adjust room amount. Up and down adjust depth. Hold Shift for larger steps.</span></div>;
}
