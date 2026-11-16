import type {ReactNode} from 'react';
import {haptic} from './haptics';
export default function ThemedSwitch({label,checked,onChange,disabled=false,ariaLabel,labelVisual}:{label:string;checked:boolean;onChange:(checked:boolean)=>void;disabled?:boolean;ariaLabel?:string;labelVisual?:ReactNode}) {
 return <label className="themed-switch"><span aria-hidden={labelVisual?true:undefined}>{labelVisual??label}</span><input type="checkbox" role="switch" aria-label={ariaLabel??(labelVisual?label:undefined)} disabled={disabled} checked={checked} onChange={e=>{haptic('toggle',label);onChange(e.target.checked);}}/><span className="switch-track" aria-hidden="true"/></label>;
}
