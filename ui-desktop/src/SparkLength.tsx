import {useId,type CSSProperties} from 'react';
export default function SparkLength({value,onChange,disabled=false}:{value:number;onChange:(value:number)=>void;disabled?:boolean}){
 const id=useId();return <label className="spark-length" htmlFor={id}><span>Length<output htmlFor={id}>{value} songs</output></span><input id={id} style={{'--range-fill':`${(value-10)/90*100}%`} as CSSProperties} type="range" min={10} max={100} step={1} value={value} disabled={disabled} aria-label="Sparklist length" aria-valuetext={`${value} songs`} onChange={e=>onChange(Number(e.target.value))}/></label>;
}
