import { useEffect, useRef, useState, type CSSProperties } from "react";
import type {Playback} from "./types";
import { duration as formatTime } from "./components";
export default function PlaybackProgress({trackId,position,duration,seek,showTime=true,transition}:{showTime?:boolean;transition?:Playback["dj_transition"];trackId?:number;position:number;duration:number;seek:(position:number)=>Promise<void>}) {
  const [draft,setDraft]=useState<number|null>(null);
  const [pending,setPending]=useState(false);
  const [error,setError]=useState("");
  const input=useRef<HTMLInputElement>(null);
  const dragging=useRef(false);
  const operation=useRef(0);
  const maximum=Math.max(0,Number.isFinite(duration)?duration:0);
  const value=Math.max(0,Math.min(maximum,draft??position));
  const percent=maximum>0?value/maximum*100:0;
  useEffect(()=>{operation.current++;setDraft(null);setPending(false);setError("");dragging.current=false;},[trackId]);
  async function commit(next:number) {
    const revision=++operation.current;setPending(true);setError("");
    try { await seek(Math.max(0,Math.min(maximum,next))); }
    catch { if(revision===operation.current)setError("Could not seek. Try again."); }
    finally {if(revision===operation.current){setPending(false);setDraft(null);}}
  }
  return <div className="playback-progress" data-seeking={pending || dragging.current || undefined}>
    <div className="playback-rail-slot"><div className="progress-rail" aria-hidden="true"><span style={{transform:`scaleX(${percent/100})`}}/></div>
    <input ref={input} className="seek" type="range" aria-label="Playback position"
      aria-valuetext={`${formatTime(value)} of ${formatTime(maximum)}`}
      min={0} max={Math.max(1,maximum)} step={.1} value={value}
      disabled={!trackId || maximum===0} style={{"--progress":`${percent}%`} as CSSProperties}
      title={`${formatTime(value)} / ${formatTime(maximum)}`}
      onPointerDown={()=>{dragging.current=true;setDraft(value);}}
      onChange={e=>{const next=Number(e.currentTarget.value);setDraft(next);if(!dragging.current)void commit(next);}}
      onPointerUp={e=>{if(dragging.current){dragging.current=false;void commit(Number(e.currentTarget.value));}}}
      onPointerCancel={()=>{dragging.current=false;setDraft(null);}}
      onBlur={()=>{if(dragging.current){dragging.current=false;if(input.current)void commit(Number(input.current.value));}}}
    />
    {transition&&(transition.status==='scheduled'||transition.active)&&transition.seconds>0&&maximum>0&&<span className="transition-region" style={{left:`${Math.min(100,transition.outgoing_cue/maximum*100)}%`,width:`${Math.max(.25,Math.min(100-transition.outgoing_cue/maximum*100,(transition.source_seconds??transition.seconds)/maximum*100))}%`}}><button aria-label={`DJ transition ${transition.active?'active':'scheduled'} at ${formatTime(transition.outgoing_cue)}, ${transition.seconds.toFixed(1)} seconds`} title={`DJmode ${transition.active?'mixing':'starts'} · ${formatTime(transition.outgoing_cue)} · ${transition.seconds.toFixed(1)} s`} /></span>}
    </div>{showTime&&trackId&&<output className="playback-time">{formatTime(value)} <span>/ {formatTime(maximum)}</span></output>}
    {error && <span className="seek-error" role="alert">{error}</span>}
  </div>;
}
