import {useRef,useState,type CSSProperties} from 'react';

export default function VolumeControl({value,onChange}:{value:number;onChange:(value:number)=>void|Promise<void>}) {
  const [draft,setDraft]=useState<number|null>(null);
  const [error,setError]=useState('');
  const remembered=useRef(value>0?value:.5);
  const queued=useRef<number|null>(null);
  const sending=useRef(false);
  const volume=Math.max(0,Math.min(1,draft??value));
  async function change(next:number) {
    next=Math.max(0,Math.min(1,next));
    if(next>0)remembered.current=next;
    setDraft(next);setError('');queued.current=next;
    if(sending.current)return;
    sending.current=true;
    try {
      while(queued.current!==null){const latest=queued.current;queued.current=null;await onChange(latest);}
    } catch {queued.current=null;setError('Could not change volume. Try again.');}
    finally {sending.current=false;setDraft(null);}
  }
  return <div className="volume volume-control" style={{'--volume-level':`${volume*100}%`} as CSSProperties}>
    <button className="volume-mute" aria-label={volume===0?'Unmute':'Mute'} aria-pressed={volume===0} onClick={()=>void change(volume===0?remembered.current:0)}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M11 4 6 8H3v8h3l5 4Z"/>{volume===0?<path d="m16 9 6 6m0-6-6 6"/>:<><path d="M15 8a6 6 0 0 1 0 8"/>{volume>.5&&<path d="M18 5a10 10 0 0 1 0 14"/>}</>}</svg>
    </button>
    <input type="range" aria-label="Volume" aria-valuetext={`${Math.round(volume*100)} percent`} min={0} max={1} step={.01} value={volume} onChange={e=>void change(Number(e.currentTarget.value))}/>
    <output aria-hidden="true">{Math.round(volume*100)}<span>%</span></output>
    {error&&<small className="volume-error" role="alert">{error}</small>}
  </div>;
}
