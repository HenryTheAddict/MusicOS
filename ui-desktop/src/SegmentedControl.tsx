import type {CSSProperties,ReactNode} from 'react';
export type SegmentedOption<T extends string>={value:T;label:string;icon?:ReactNode;disabled?:boolean};
export default function SegmentedControl<T extends string>({options,value,onChange,label,className}:{options:SegmentedOption<T>[];value:T;onChange:(value:T)=>void;label:string;className?:string}){
 const selected=options.findIndex(option=>option.value===value);
 return <div className={className?`segmented-control ${className}`:'segmented-control'} role="group" aria-label={label} data-selected={selected>=0} style={{'--segments':options.length,'--segment':Math.max(0,selected)} as CSSProperties}>{options.map(option=><button key={option.value} type="button" aria-pressed={option.value===value} disabled={option.disabled} onClick={()=>{if(option.value!==value)onChange(option.value);}}>{option.icon}<span>{option.label}</span></button>)}</div>;
}
