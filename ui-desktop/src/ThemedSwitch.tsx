import {haptic} from './haptics';
export default function ThemedSwitch({label,checked,onChange,disabled=false,ariaLabel}:{label:string;checked:boolean;onChange:(checked:boolean)=>void;disabled?:boolean;ariaLabel?:string}) {
 return <label className="themed-switch"><span>{label}</span><input type="checkbox" role="switch" aria-label={ariaLabel} disabled={disabled} checked={checked} onChange={e=>{haptic('toggle',label);onChange(e.target.checked);}}/><span className="switch-track" aria-hidden="true"/></label>;
}
