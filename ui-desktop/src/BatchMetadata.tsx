import {useViewActive} from './ViewScope';
import {invalidateDisplay} from './displayData';
import {Icon} from './components';
import FileWriteOption from './FileWriteOption';
import ThemedChoice from './ThemedChoice';
import {Cover} from './components';
import {useEffect,useMemo,useRef,useState} from "react";
import {useVirtualizer} from "@tanstack/react-virtual";
import {call,errorText} from "./bridge";
import type {Track,Album} from "./types";
const fields=[['artist','Artist'],['album','Album'],['album_artist','Album artist'],['genre','Genre'],['year','Year'],['track_number','Track number'],['disc_number','Disc number'],['title','Title']] as const;
type Field=typeof fields[number][0];
export default function BatchMetadata({tracks,albums=[],initialIds=[],onRefresh,onDirtyChange,onBusyChange}:{albums?:Album[];tracks:Track[];initialIds?:number[];onRefresh:()=>Promise<void>;onDirtyChange?:(dirty:boolean)=>void;onBusyChange?:(busy:boolean)=>void}) {
  const active=useViewActive();
  const [detail,setDetail]=useState(initialIds.length>0),[outdated,setOutdated]=useState(false);
  useEffect(()=>{if(initialIds.length){setSelected(new Set(initialIds));setDetail(true);setView('songs');setReviewing(false);}},[initialIds.join(',')]);
  useEffect(()=>{const changed=(e:Event)=>{if(!['analysis','favorites'].includes((e as CustomEvent).detail?.reason))setOutdated(true);};window.addEventListener('musicos-library-changed',changed);return()=>window.removeEventListener('musicos-library-changed',changed);},[]);
  const [view,setView]=useState<'albums'|'songs'>(initialIds.length?'songs':'albums');
  const [query,setQuery]=useState("");
  const [selected,setSelected]=useState(()=>new Set(initialIds));
  const [enabled,setEnabled]=useState<Set<Field>>(new Set());
  const [values,setValues]=useState<Partial<Record<Field,string>>>({});
  const [write,setWrite]=useState(false),[fileReady,setFileReady]=useState(true),[revisions,setRevisions]=useState<[number,number][]>([]);
  const [reviewing,setReviewing]=useState(false),[cleared,setCleared]=useState<Set<Field>>(new Set());
  const [busy,setBusy]=useState(false),[message,setMessage]=useState(""),[error,setError]=useState("");
  useEffect(()=>onDirtyChange?.(enabled.size>0),[enabled.size,onDirtyChange]);
  useEffect(()=>{onBusyChange?.(busy);return()=>onBusyChange?.(false);},[busy,onBusyChange]);
  useEffect(()=>{if(selected.size!==1)setEnabled(old=>{if(!old.has("title"))return old;const next=new Set(old);next.delete("title");return next;});},[selected.size]);
  const scroll=useRef<HTMLDivElement>(null);
  const matching=useMemo(()=>tracks.filter(t=>`${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(query.trim().toLowerCase())),[tracks,query]);
  const matchingAlbums=useMemo(()=>albums.filter(album=>
    `${album.title} ${album.artist}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())
  ),[albums,query]);
  const pickLabel=view==='albums'?'albums':'songs';
  const hasMatches=view==='albums'?matchingAlbums.length>0:matching.length>0;
  function selectMatches(){
    const ids=view==='albums'?matchingAlbums.flatMap(album=>album.track_ids):matching.map(track=>track.id);
    setSelected(previous=>new Set([...previous,...ids]));
    setReviewing(false);
  }
  const rowHeight=3*parseFloat(getComputedStyle(document.documentElement).fontSize);
  const rows=useVirtualizer({count:matching.length,getScrollElement:()=>scroll.current,estimateSize:()=>rowHeight,overscan:4,enabled:active});
  const selectedTracks=tracks.filter(t=>selected.has(t.id));
  function shared(field:Field){const values=[...new Set(selectedTracks.map(t=>String(t[field]??'')))];return values.length===1?values[0]:values.length>1?'Multiple values':'';}
  function mode(field:Field,value:string){setReviewing(false);setEnabled(old=>{const next=new Set(old);value==='unchanged'?next.delete(field):next.add(field);return next;});setCleared(old=>{const next=new Set(old);value==='clear'?next.add(field):next.delete(field);return next;});setValues(old=>({...old,[field]:value==='clear'?['genre','album_artist'].includes(field)?'':'0':shared(field)==='Multiple values'?'':shared(field)}));}
  function toggle(id:number){setSelected(old=>{const next=new Set(old);next.has(id)?next.delete(id):next.add(id);return next;});setMessage("");setReviewing(false);}
  async function save(){
    setBusy(true);setError("");setMessage("");
    try {
      const patch:Record<string,string|number>={};
      for(const field of enabled){const value=(values[field]??"").trim();if(['year','track_number','disc_number'].includes(field)){if(!/^\d{1,4}$/.test(value))throw new Error("Enter a number between 0 and 9999.");patch[field]=Number(value);}else{if(!["genre","album_artist"].includes(field)&&!value)throw new Error("Artist, album and title cannot be blank.");patch[field]=value;}}
      const count=await call("metadata_edit",{ids:[...selected],patch,writeFiles:write,revisions});invalidateDisplay();await onRefresh();setMessage(`Saved metadata for ${count} ${count===1?'song':'songs'}.`);setEnabled(new Set());setCleared(new Set());setWrite(false);setReviewing(false);
    }catch(e){setError(errorText(e));}finally{setBusy(false);}
  }
  return <section className="batch-metadata" aria-label="Edit library metadata">
    <header><h2>Edit metadata</h2><p>Choose songs and fields to change. Review before saving; changes stay in musicOS unless you choose to write file tags.</p></header>
    <div className="metadata-split" data-detail={detail}><aside className="metadata-browser"><div className="size-options" role="group" aria-label="Edit selection"><button aria-pressed={view==='albums'} onClick={()=>setView('albums')}>Albums</button><button aria-pressed={view==='songs'} onClick={()=>setView('songs')}>Songs</button></div><fieldset disabled={busy} className="metadata-selection"><legend>Choose songs</legend>
      <label className="metadata-filter">Find {pickLabel}<input type="search" aria-label={`Find ${pickLabel} to edit`} value={query}
        onChange={event=>{setQuery(event.target.value);scroll.current?.scrollTo({top:0});}} placeholder={view==='albums'?'Album or artist':'Title, artist or album'}/></label>
      <div className="selection-actions">
        <output aria-live="polite">{selected.size} selected</output>
        <button type="button" disabled={!hasMatches} onClick={selectMatches}>Select matching {pickLabel}</button>
        <button type="button" disabled={!selected.size} onClick={()=>{setSelected(new Set());setReviewing(false);}}>Clear selection</button>
      </div>
      {view==='albums'?<div className="metadata-album-list">
        {matchingAlbums.map(album=>{
          const count=album.track_ids.filter(id=>selected.has(id)).length;
          const included=count===album.track_ids.length;
          return <button key={album.id} aria-pressed={included} data-partial={count>0&&!included||undefined} onClick={()=>{
            setSelected(previous=>{
              const next=new Set(previous),remove=album.track_ids.every(id=>next.has(id));
              for(const id of album.track_ids)remove?next.delete(id):next.add(id);
              return next;
            });
            setReviewing(false);
          }}>
            <Cover src={album.cover} title=""/>
            <span><strong>{album.title}</strong><small>{album.artist} · {album.track_ids.length} {album.track_ids.length===1?'song':'songs'}</small></span>
            {count>0&&<span className="metadata-partial-count">{count} selected</span>}
          </button>;
        })}
        {!matchingAlbums.length&&<p role="status">No matching albums.</p>}
      </div>:<div className="metadata-song-list" ref={scroll} style={{height:Math.min(16/3,Math.max(2,matching.length))*rowHeight}}><div style={{height:rows.getTotalSize(),position:'relative'}}>{rows.getVirtualItems().map(row=>{const track=matching[row.index];return <label key={track.id} className="metadata-song" style={{position:'absolute',top:0,left:0,right:0,height:row.size,transform:`translateY(${row.start}px)`}}><input type="checkbox" aria-label={`Select ${track.title} by ${track.artist}`} checked={selected.has(track.id)} onChange={()=>toggle(track.id)}/><span><strong>{track.title}</strong><small>{track.artist} · {track.album}</small></span></label>;})}</div>{!matching.length&&<p>No matching songs.</p>}</div>}
    </fieldset><button className="metadata-continue" disabled={!selected.size} onClick={()=>setDetail(true)}>Edit {selected.size} songs ›</button></aside><div className="metadata-inspector"><button className="metadata-back" onClick={()=>setDetail(false)}><Icon name="Back"/>Back to albums & songs</button>
    <fieldset disabled={busy||reviewing} className="metadata-fields"><legend>Choose changes</legend>{fields.map(([field,label])=>{const single=field==='title'&&selected.size!==1;return <div className="metadata-field" data-editing={enabled.has(field)&&!cleared.has(field)&&!single} key={field}><label>{label}<small>{single?'Select one song to edit its title':shared(field)||'Not set'}</small></label><ThemedChoice label={`${label} change`} disabled={single} value={!enabled.has(field)?'unchanged':cleared.has(field)?'clear':'set'} onChange={value=>mode(field,value)} options={[{value:'unchanged',label:'Leave unchanged'},{value:'set',label:'Set value'},...(!['title','artist','album'].includes(field)?[{value:'clear',label:'Clear value'}]:[])]}/><input hidden={!enabled.has(field)||single||cleared.has(field)} aria-label={`New ${label.toLowerCase()}`} disabled={!enabled.has(field)||single||cleared.has(field)} type={['year','track_number','disc_number'].includes(field)?'number':'text'} min={0} max={9999} maxLength={512} placeholder={shared(field)||'Not set'} value={values[field]??''} onChange={e=>setValues(old=>({...old,[field]:e.target.value}))}/></div>;})}</fieldset>
    {reviewing&&<section className="metadata-bulk-review" aria-label="Review bulk changes"><h2>Review {selected.size} selected songs</h2>{[...enabled].map(field=><p key={field}><strong>{field.replaceAll('_',' ')}</strong>: {shared(field)||'Not set'} → {cleared.has(field)?'Clear value':values[field]||'Empty value'}</p>)}<p>Only these fields change. Undo is available in History.</p><details className="tag-advanced" onToggle={event=>{if(!event.currentTarget.open)setWrite(false);}}><summary>Advanced · original file tags</summary><FileWriteOption ids={[...selected]} value={write} onChange={setWrite} onReady={setFileReady}/></details></section>}
    <footer className="metadata-action-footer"><output>{selected.size} songs · {enabled.size} {enabled.size===1?'field':'fields'}</output>{reviewing&&<><button aria-label={`Apply changes to ${selected.size} ${selected.size===1?'song':'songs'}`} disabled={busy||outdated||(write&&!fileReady)} onClick={()=>void save()}><Icon name="File"/>Apply changes</button><button disabled={busy} onClick={()=>setReviewing(false)}><Icon name="Back"/>Back</button></>}{!reviewing&&<button disabled={busy||!selected.size||!enabled.size||(enabled.has('title')&&selected.size!==1)} onClick={()=>void call('metadata_files',{action:'inspect',ids:[...selected]}).then(r=>{setRevisions((r.values??[]).map(v=>[Number(v.id),Number(v.revision)]));setWrite(false);setOutdated(false);setReviewing(true);}).catch(e=>setError(errorText(e)))}>Review changes to {selected.size} {selected.size===1?'song':'songs'}</button>}{message&&<p role="status">{message}</p>}{error&&<p className="inline-error" role="alert">{error}</p>}</footer>{outdated&&reviewing&&<p role="status">Your selection changed since review. Go back and review again before applying.</p>}</div></div>
  </section>;
}
