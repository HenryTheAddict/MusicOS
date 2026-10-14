export default function ThemedSwitch({label,checked,onChange,disabled=false}:{label:string;checked:boolean;onChange:(checked:boolean)=>void;disabled?:boolean}) {
 return <label className="themed-switch"><span>{label}</span><input type="checkbox" role="switch" disabled={disabled} checked={checked} onChange={e=>onChange(e.target.checked)}/><span className="switch-track" aria-hidden="true"/></label>;
}
