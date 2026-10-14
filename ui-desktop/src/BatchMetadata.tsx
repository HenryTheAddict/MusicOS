import {useEffect,useMemo,useRef,useState} from "react";
import {useVirtualizer} from "@tanstack/react-virtual";
import {call,errorText} from "./bridge";
import type {Track} from "./types";
const fields=[['artist','Artist'],['album','Album'],['genre','Genre'],['year','Year'],['track_number','Track number'],['disc_number','Disc number'],['title','Title']] as const;
type Field=typeof fields[number][0];
export default function BatchMetadata({tracks,initialIds=[],onRefresh}:{tracks:Track[];initialIds?:number[];onRefresh:()=>Promise<void>}) {
  const [query,setQuery]=useState("");
  const [selected,setSelected]=useState(()=>new Set(initialIds));
  const [enabled,setEnabled]=useState<Set<Field>>(new Set());
  const [values,setValues]=useState<Partial<Record<Field,string>>>({});
  const [reviewing,setReviewing]=useState(false),[cleared,setCleared]=useState<Set<Field>>(new Set());
  const [busy,setBusy]=useState(false),[message,setMessage]=useState(""),[error,setError]=useState("");
  useEffect(()=>{if(selected.size!==1)setEnabled(old=>{if(!old.has("title"))return old;const next=new Set(old);next.delete("title");return next;});},[selected.size]);
  const scroll=useRef<HTMLDivElement>(null);
  const matching=useMemo(()=>tracks.filter(t=>`${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(query.trim().toLowerCase())),[tracks,query]);
  const rowHeight=3*parseFloat(getComputedStyle(document.documentElement).fontSize);
  const rows=useVirtualizer({count:matching.length,getScrollElement:()=>scroll.current,estimateSize:()=>rowHeight,overscan:4});
  const selectedTracks=tracks.filter(t=>selected.has(t.id));
  function shared(field:Field){const values=[...new Set(selectedTracks.map(t=>String(t[field]??'')))];return values.length===1?values[0]:values.length>1?'Multiple values':'';}
  function mode(field:Field,value:string){setReviewing(false);setEnabled(old=>{const next=new Set(old);value==='unchanged'?next.delete(field):next.add(field);return next;});setCleared(old=>{const next=new Set(old);value==='clear'?next.add(field):next.delete(field);return next;});setValues(old=>({...old,[field]:value==='clear'?field==='genre'?'':'0':shared(field)==='Multiple values'?'':shared(field)}));}
  function toggle(id:number){setSelected(old=>{const next=new Set(old);next.has(id)?next.delete(id):next.add(id);return next;});setMessage("");}
  async function save(){
    setBusy(true);setError("");setMessage("");
    try {
      const patch:Record<string,string|number>={};
      for(const field of enabled){const value=(values[field]??"").trim();if(['year','track_number','disc_number'].includes(field)){if(!/^\d{1,4}$/.test(value))throw new Error("Enter a number between 0 and 9999.");patch[field]=Number(value);}else{if(field!=="genre"&&!value)throw new Error("Artist, album and title cannot be blank.");patch[field]=value;}}
      const count=await call("metadata_edit",{ids:[...selected],patch});await onRefresh();setMessage(`Saved metadata for ${count} ${count===1?'song':'songs'}.`);setEnabled(new Set());setCleared(new Set());setReviewing(false);
    }catch(e){setError(errorText(e));}finally{setBusy(false);}
  }
  return <section className="batch-metadata" aria-label="Edit library metadata">
    <header><h2>Edit metadata</h2><p>Select songs, then check the fields you want to change. Unchecked fields stay as they are. Changes stay in MusicOS; original file tags are preserved.</p></header>
    <fieldset disabled={busy} className="metadata-selection"><legend>Choose songs</legend>
      <label className="metadata-filter">Find songs<input aria-label="Find songs to edit" value={query} onChange={e=>{setQuery(e.target.value);scroll.current?.scrollTo({top:0});}} placeholder="Title, artist or album"/></label>
      <div className="selection-actions"><button type="button" onClick={()=>setSelected(old=>new Set([...old,...matching.map(t=>t.id)]))}>Select matching songs</button><button type="button" onClick={()=>setSelected(new Set())}>Clear selection</button><output>{selected.size} selected</output></div>
      <div className="metadata-song-list" ref={scroll} style={{height:Math.min(16/3,Math.max(2,matching.length))*rowHeight}}><div style={{height:rows.getTotalSize(),position:'relative'}}>{rows.getVirtualItems().map(row=>{const track=matching[row.index];return <label key={track.id} className="metadata-song" style={{position:'absolute',top:0,left:0,right:0,height:row.size,transform:`translateY(${row.start}px)`}}><input type="checkbox" aria-label={`Select ${track.title} by ${track.artist}`} checked={selected.has(track.id)} onChange={()=>toggle(track.id)}/><span><strong>{track.title}</strong><small>{track.artist} · {track.album}</small></span></label>;})}</div>{!matching.length&&<p>No matching songs.</p>}</div>
    </fieldset>
    <fieldset disabled={busy||reviewing} className="metadata-fields"><legend>Choose changes</legend>{fields.map(([field,label])=>{const single=field==='title'&&selected.size!==1;return <div className="metadata-field" key={field}><label>{label}<small>{single?'Select one song to edit its title':shared(field)||'Not set'}</small></label><select aria-label={`${label} change`} disabled={single} value={!enabled.has(field)?'unchanged':cleared.has(field)?'clear':'set'} onChange={e=>mode(field,e.target.value)}><option value="unchanged">Leave unchanged</option><option value="set">Set value</option>{!['title','artist','album'].includes(field)&&<option value="clear">Clear value</option>}</select><input aria-label={`New ${label.toLowerCase()}`} disabled={!enabled.has(field)||single||cleared.has(field)} type={['year','track_number','disc_number'].includes(field)?'number':'text'} min={0} max={9999} maxLength={512} placeholder={shared(field)||'Not set'} value={values[field]??''} onChange={e=>setValues(old=>({...old,[field]:e.target.value}))}/></div>;})}</fieldset>
    {reviewing&&<section className="metadata-bulk-review" aria-label="Review bulk changes"><h2>Review {selected.size} selected songs</h2>{[...enabled].map(field=><p key={field}><strong>{field.replaceAll('_',' ')}</strong>: {shared(field)||'Not set'} → {cleared.has(field)?'Clear value':values[field]||'Empty value'}</p>)}<p>Only these fields change. Original file tags stay intact. Undo is available in History.</p><button disabled={busy} onClick={()=>void save()}>Apply changes to {selected.size} {selected.size===1?'song':'songs'}</button><button disabled={busy} onClick={()=>setReviewing(false)}>Back to editing</button></section>}
    <footer>{!reviewing&&<button disabled={busy||!selected.size||!enabled.size||(enabled.has('title')&&selected.size!==1)} onClick={()=>setReviewing(true)}>Review changes to {selected.size} {selected.size===1?'song':'songs'}</button>}{message&&<p role="status">{message}</p>}{error&&<p className="inline-error" role="alert">{error}</p>}</footer>
  </section>;
}
