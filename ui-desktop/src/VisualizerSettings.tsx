import {useEffect,useRef,useState} from 'react';
import {Icon} from './components';
import VisualizerOptions from './VisualizerOptions';
import type {VisualizerPreferences} from './visualizer';

/** A nested player dialog keeps chrome visible and owns Escape before the player. */
export default function VisualizerSettings({value,onChange,fallback}:{value:VisualizerPreferences;onChange:(value:VisualizerPreferences)=>void;fallback:boolean}){
 const [open,setOpen]=useState(false),dialog=useRef<HTMLDialogElement>(null),trigger=useRef<HTMLButtonElement>(null);
 useEffect(()=>{if(!open)return;const node=dialog.current!;node.showModal();return()=>{node.close();if(trigger.current?.isConnected)trigger.current.focus({preventScroll:true});};},[open]);
 return <><button ref={trigger} className="visualizer-settings-trigger" aria-label="Visualizer options" aria-haspopup="dialog" aria-expanded={open} onClick={()=>setOpen(true)}><Icon name="Settings"/></button>{open&&<dialog ref={dialog} className="visualizer-settings" aria-labelledby="visualizer-settings-title" onCancel={e=>{e.preventDefault();e.stopPropagation();setOpen(false);}}><header><h2 id="visualizer-settings-title">Visualizer</h2><button className="icon-button" aria-label="Close visualizer options" onClick={()=>setOpen(false)}><Icon name="controlClose"/></button></header><VisualizerOptions picker={false} value={value} onChange={onChange} fallback={fallback}/></dialog>}</>;
}
