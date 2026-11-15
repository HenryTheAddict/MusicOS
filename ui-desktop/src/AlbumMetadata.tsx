import {useEffect,useRef,useState} from 'react';
import {open} from '@tauri-apps/plugin-dialog';
import {artwork as imageUrl,call,errorText,native} from './bridge';
import {displayData,invalidateDisplay} from './displayData';
import {Cover,Icon} from './components';
import type {Album,AlbumReview,Track} from './types';
import {KeepView,useViewActive} from './ViewScope';
import FileWriteOption from './FileWriteOption';
import LyricsEditor from './LyricsEditor';
import ManagementTabs from './ManagementTabs';
import MetadataFields,{displayMetadata,fieldLabel,metadataFields,songCount,tagPatch,type MetadataDraft,type MetadataField,type MetadataPatch} from './MetadataFields';

export type InspectorTab = 'cover' | 'details' | 'songs';
type Row = AlbumReview['rows'][number];
type Change = {id:number; title:string; field:MetadataField; before:unknown; after:unknown};
type Position = {track_id:number;disc_number?:number;track_number?:number};
type ImportedCover = {id:string;path:string;size:number[]};
type Review = {kind:'cover'; token:string; source?:Row; path:string; imported?:ImportedCover; ids:number[]}
  | {kind:'positions';token:string;changes:Change[];positions:Position[];ids:number[]}
  | {kind:'tags'; token:string; patch:MetadataPatch; changes:Change[]; ids:number[]; revisions:[number,number][]}
  | {kind:'suggestions'; token:string; changes:Change[]; ids:number[]};

const albumFields = ['album','album_artist','year','genre'] as const;
const proposedFields = ['title','artist','album','album_artist','year','track_number','disc_number'] as const;

function suggestedChanges(rows: Row[]) {
  return rows.flatMap(row => proposedFields.flatMap(field => {
    const after = row.candidate?.[field];
    const before = row.current[field];
    return after != null && displayMetadata(after) !== displayMetadata(before)
      ? [{id:row.track_id,title:String(row.current.title),field,before,after}] : [];
  }));
}

const changeKey = (change: Change) => `${change.id}:${change.field}`;
function changeGroups(changes: Change[]) {
  const groups = new Map<string,Change[]>();
  for (const change of changes) {
    const key = JSON.stringify([change.field,change.before,change.after]);
    const group = groups.get(key) ?? [];
    group.push(change);
    groups.set(key,group);
  }
  return [...groups.values()];
}

function ChangeList({changes,choices,onChoose}: {
  changes:Change[]; choices?:Set<string>; onChoose?:(group:Change[],checked:boolean)=>void;
}) {
  return <div className="tag-changes">
    {changeGroups(changes).map(group => {
      const {field,before,after} = group[0];
      return <div className="tag-change" key={changeKey(group[0])}>
        <label>
          {choices && <input type="checkbox" checked={group.every(change=>choices.has(changeKey(change)))}
            aria-label={group.length===1? `Apply ${fieldLabel[field].toLowerCase()} for ${group[0].title}` : `Apply ${fieldLabel[field].toLowerCase()} to ${songCount(group.length)}`}
            onChange={event=>onChoose?.(group,event.target.checked)}/>}
          <strong>{fieldLabel[field]}</strong>
          <span><span>{displayMetadata(before)}</span><span aria-hidden="true">→</span><strong>{displayMetadata(after)}</strong></span>
        </label>
        {group.length===1 ? <small>{group[0].title}</small> : <details>
          <summary>{songCount(group.length)} affected</summary>
          <ul>{group.map(change=><li key={change.id}>{change.title}</li>)}</ul>
        </details>}
      </div>;
    })}
  </div>;
}

function MatchDetails({rows,disabled,onIgnore}: {rows:Row[];disabled:boolean;onIgnore:(row:Row)=>void}) {
  const matches = rows.filter(row=>row.candidate);
  return !!matches.length && <details className="tag-matches">
    <summary>Recording and release matches</summary>
    {matches.map(row => <article key={row.track_id}>
      <strong>{String(row.current.title)}</strong>
      {typeof row.candidate?.confidence === 'number' && <small>{Math.round(row.candidate.confidence*100)}% match</small>}
      <div className="metadata-source-actions">
        {row.candidate?.recording_mbid != null && <button onClick={()=>void call('open_provider',{url:`https://musicbrainz.org/recording/${row.candidate?.recording_mbid}`})}>Recording source ↗</button>}
        {row.candidate?.release_mbid != null && <button onClick={()=>void call('open_provider',{url:`https://musicbrainz.org/release/${row.candidate?.release_mbid}`})}>Release source ↗</button>}
        <button disabled={disabled} onClick={()=>onIgnore(row)}>Ignore suggestion</button>
      </div>
    </article>)}
  </details>;
}

function coverResolution(size?:number[]) {
  return size ? size.join(' × ') : 'Resolution unknown';
}

function CoverComparison({current,proposed,lowerResolution,identical}: {
  current:{path?:string|null;size?:number[]};proposed?:{path:string;size?:number[];local?:boolean};lowerResolution?:boolean;identical?:boolean;
}) {
  return <div className="tag-cover-comparison" data-single={!proposed||undefined}>
    <figure>
      {current.path?<img className="cover" src={imageUrl(current.path)} alt="Current cover"/>:<Cover title=""/>}
      <figcaption><strong>Current cover</strong><span>{current.path?coverResolution(current.size):'No cover saved'}</span>
        {identical&&<span className="tag-cover-match">Suggested cover matches</span>}
      </figcaption>
    </figure>
    {proposed&&<figure>
      <img className="cover" src={imageUrl(proposed.path)} alt={proposed.local?'Imported cover':'Suggested cover'}/>
      <figcaption><strong>{proposed.local?'Imported cover':'Suggested cover'}</strong><span>{coverResolution(proposed.size)} · {proposed.local?'Local image':'Cover Art Archive'}</span>
        {lowerResolution&&<span className="tag-cover-quality">Lower resolution</span>}
      </figcaption>
      {proposed.local&&<div className="tag-crop-preview"><img src={imageUrl(proposed.path)} alt="Square crop"/><small>In the library</small></div>}
    </figure>}
  </div>;
}

export default function AlbumMetadata({album,track,tab,lyrics,lyricsRequest=0,epoch,playing,onCheck,onTab,onSong,onLyrics,onDirty,onSaving,onSaved,onRefresh,onComparison}: {
  album:Album; track?:Track; tab:InspectorTab; lyrics:boolean; lyricsRequest?:number; epoch:number; playing:boolean;
  onCheck?:()=>void; onTab:(tab:InspectorTab)=>void; onSong:(id:number)=>void; onLyrics:(show:boolean)=>void;
  onDirty:(dirty:boolean)=>void; onSaving:(saving:boolean)=>void;
  onSaved:(message:string)=>void; onRefresh:()=>Promise<void>; onComparison?:(result:AlbumReview)=>void;
}) {
  const active = useViewActive();
  const [comparison,setComparison] = useState<AlbumReview|null>(null);
  const [drafts,setDrafts] = useState<{album:MetadataDraft;song:MetadataDraft}>({album:{},song:{}});
  const [choices,setChoices] = useState<Set<string>>(new Set());
  const [path,setPath] = useState('');
  const [imported,setImported] = useState<ImportedCover>();
  const [importing,setImporting] = useState(false);
  const [positions,setPositions] = useState<Record<number,{disc_number:string;track_number:string}>>({});
  const [order,setOrder] = useState<number[]>([]);
  const [review,setReview] = useState<Review|null>(null);
  const [lyricDirty,setLyricDirty] = useState(false);
  const [lyricBusy,setLyricBusy] = useState(false);
  const [write,setWrite] = useState(false);
  const [fileReady,setFileReady] = useState(false);
  const [loading,setLoading] = useState(true);
  const [saving,setSaving] = useState(false);
  const [error,setError] = useState('');
  const [loadError,setLoadError] = useState('');
  const [notice,setNotice] = useState('');
  const [reload,setReload] = useState(0);
  const reviewHeading = useRef<HTMLHeadingElement>(null);
  const dropZone = useRef<HTMLElement>(null);
  const imageInput = useRef<HTMLInputElement>(null);
  const importVersion = useRef(0);
  const rows = comparison?.album_id===album.id ? comparison.rows : [];
  const scope = tab==='songs' && track ? 'song' : 'album';
  const scopedRows = scope==='song' ? rows.filter(row=>row.track_id===track?.id) : rows;
  const draft = drafts[scope];
  const proposals = suggestedChanges(scopedRows);
  const selected = proposals.filter(change=>choices.has(changeKey(change)));
  const artwork = [...new Map(rows.flatMap(row => {
    if(row.art_comparison==='identical')return [];
    const cover = row.candidate?.artwork_path;
    return typeof cover==='string' && row.art_available!==false && row.art_comparison!=='identical' && cover!==comparison?.cover ? [[cover,row] as const] : [];
  })).entries()];
  const proposedCover = artwork.find(([cover])=>cover===path) ?? (artwork.length===1 ? artwork[0] : undefined);
  const lowerResolution = (size?:number[]) => !!comparison?.resolution?.some((pixels,axis)=>size && pixels>size[axis]);
  const currentCover={path:comparison?.cover??album.cover,size:comparison?.resolution};
  const identicalCover=rows.some(row=>row.art_comparison==='identical');
  const differentCrop=proposedCover?.[1].art_comparison==='different_crop';
  const failed=rows.filter(row=>row.status==='failed').length;
  const unmatched=rows.filter(row=>row.status==='unmatched').length;
  const naturalRows=rows.filter(row=>!order.includes(row.track_id));
  const orderedRows=[...order.flatMap(id=>rows.find(row=>row.track_id===id)??[]),...naturalRows];
  const positionOf=(row:Row)=>positions[row.track_id]??{
    disc_number:String(row.current.disc_number||''),track_number:String(row.current.track_number||'')};
  const positionKey=(row:Row)=>{
    const position=positionOf(row);
    return Number(position.track_number)>0?`${Number(position.disc_number)||1}:${Number(position.track_number)}`:'';
  };
  const counts=new Map<string,number>();
  for(const row of rows){const key=positionKey(row);if(key)counts.set(key,(counts.get(key)??0)+1);}
  const duplicatePositions=[...counts.values()].some(count=>count>1);
  const missingPositions=rows.some(row=>!positionOf(row).track_number);
  const missingTags=rows.some(row=>albumFields.some(field=>!row.current[field]));
  const missingTab=!currentCover.path?'cover':duplicatePositions||missingPositions?'songs':missingTags?'details':null;
  const dirty = Object.values(drafts).some(values=>Object.keys(values).length>0) || choices.size>0 || !!path || !!imported || !!Object.keys(positions).length || !!order.length || lyricDirty || !!review;
  useEffect(()=>onDirty(dirty),[dirty,onDirty]);
  useEffect(()=>onSaving(saving||lyricBusy),[saving,lyricBusy,onSaving]);

  useEffect(()=>{
    if (!active) return;
    let current = true;
    setLoading(true);
    setLoadError('');
    void displayData('album_metadata',{request:{action:'detail',album_id:album.id}},true).then(result=>{
      if (!current) return;
      if (!result.album) throw new Error('The album comparison is unavailable.');
      if (comparison && comparison.token!==result.album.token) {
        if (review || choices.size || path) setNotice('This album changed. Check the updated values and review your changes again.');
        setChoices(new Set());
        setPath('');
        setReview(null);
        setWrite(false);
      }
      setComparison(result.album);
      onComparison?.(result.album);
    }).catch(error=>{if(current)setLoadError(errorText(error));}).finally(()=>{if(current)setLoading(false);});
    return ()=>{current=false;};
  },[active,album.id,epoch,reload]);

  useEffect(()=>{
    setReview(null);
    setWrite(false);
  },[tab,lyrics]);
  useEffect(()=>{if(review)reviewHeading.current?.focus({preventScroll:true});},[review]);
  useEffect(()=>{
    if(!active)return;
    const zone=dropZone.current;
    function drop(event:Event){
      const paths=(event as CustomEvent<string[]>).detail;
      if(paths.length!==1){setError('Drop one image at a time.');return;}
      void importCover({image_path:paths[0]});
    }
    zone?.addEventListener('musicos-drop-cover',drop);
    return ()=>zone?.removeEventListener('musicos-drop-cover',drop);
  },[active,tab,review]);
  useEffect(()=>()=>{importVersion.current++;},[]);

  async function importCover(input:{image_path:string}|{image_data:string}){
    const version=++importVersion.current;
    setImporting(true);setError('');setNotice('');
    try{
      const result=await call('album_metadata',{request:{action:'stage_cover',...input}});
      if(importVersion.current!==version)return;
      if(!result.imported_cover)throw new Error('The cover could not be imported.');
      setImported(result.imported_cover);setPath('');
    }catch(error){if(importVersion.current===version)setError(errorText(error));}
    finally{if(importVersion.current===version)setImporting(false);}
  }
  function importFile(file?:File){
    if(!file)return;
    if(file.size>20*1024*1024){setError('Choose an image smaller than 20 MB.');return;}
    const reader=new FileReader();
    reader.onload=()=>{if(typeof reader.result==='string')void importCover({image_data:reader.result.split(',')[1]});};
    reader.onerror=()=>setError('The image could not be read.');
    reader.readAsDataURL(file);
  }
  async function chooseCover(){
    if(!native){imageInput.current?.click();return;}
    try{
      const file=await open({multiple:false,title:'Choose album artwork',filters:[{name:'Images',extensions:['jpg','jpeg','png','webp']}]});
      if(typeof file==='string')await importCover({image_path:file});
    }catch(error){setError(errorText(error));}
  }
  function movePosition(id:number,delta:number){
    const ids=orderedRows.map(row=>row.track_id),index=ids.indexOf(id),target=index+delta;
    if(target<0||target>=ids.length)return;
    [ids[index],ids[target]]=[ids[target],ids[index]];
    setOrder(ids);
  }
  function renumber(){
    const next:typeof positions={},perDisc=new Map<number,number>();
    for(const row of orderedRows){
      const position=positionOf(row),disc=Number(position.disc_number)||1;
      const number=(perDisc.get(disc)??0)+1;
      perDisc.set(disc,number);
      next[row.track_id]={...position,track_number:String(number)};
    }
    setPositions(next);
    setNotice('Review the new track numbers before saving.');
  }

  function update(field:MetadataField,value:string|undefined) {
    setDrafts(old=>{
      const next = {...old[scope]};
      if (value===undefined) delete next[field];
      else next[field]=value;
      return {...old,[scope]:next};
    });
    setError('');
  }

  function startReview(kind:Review['kind']) {
    if (!comparison || loading || loadError || !scopedRows.length) return;
    setError('');
    setNotice('');
    setWrite(false);
    if (kind==='cover' && (imported||proposedCover)) {
      setReview({kind,token:comparison.token,path:imported?.path??proposedCover![0],source:imported?undefined:proposedCover?.[1],imported,ids:rows.map(row=>row.track_id)});
    } else if(kind==='positions'){
      const changes:Change[]=[];
      const edits=orderedRows.flatMap(row=>{
        const patch:Position={track_id:row.track_id};
        for(const field of ['disc_number','track_number'] as const){
          const value=positionOf(row)[field];
          if(value&&!/^\d{1,4}$/.test(value))return [null];
          const number=Number(value);
          if(Number(row.current[field]??0)!==number){
            patch[field]=number;
            changes.push({field,id:row.track_id,title:String(row.current.title),before:row.current[field],after:number});
          }
        }
        return Object.keys(patch).length>1?[patch]:[];
      });
      if(edits.some(edit=>!edit)){setError('Use numbers between 0 and 9999.');return;}
      const positions=edits.filter(edit=>edit!==null);
      if(!positions.length){setNotice('The track numbers already match.');return;}
      setReview({kind,token:comparison.token,changes,positions,ids:positions.map(position=>position.track_id)});
    } else if (kind==='suggestions' && selected.length) {
      setReview({kind,token:comparison.token,changes:selected,ids:[...new Set(selected.map(change=>change.id))]});
    } else if (kind==='tags') {
      try {
        const patch = tagPatch(draft,scopedRows.map(row=>row.current));
        const changes = scopedRows.flatMap(row=>metadataFields.flatMap(([field])=>
          patch[field]!==undefined && displayMetadata(row.current[field])!==displayMetadata(patch[field])
            ? [{id:row.track_id,title:String(row.current.title),field,before:row.current[field],after:patch[field]}] : []));
        if (!changes.length) {
          setNotice('These values already match the library.');
          return;
        }
        const ids = [...new Set(changes.map(change=>change.id))];
        setReview({kind,token:comparison.token,patch,changes,ids,
          revisions:scopedRows.filter(row=>ids.includes(row.track_id)).map(row=>[row.track_id,Number(row.current.revision)])});
      } catch (error) { setError(errorText(error)); }
    }
  }

  async function save() {
    if (!review || saving || loading || loadError || review.token!==comparison?.token || (write&&(!fileReady||playing))) return;
    setSaving(true);
    setError('');
    try {
      let count: number;
      if (review.kind==='tags') {
        count = await call('metadata_edit',{ids:review.ids,patch:review.patch,revisions:review.revisions,writeFiles:write});
        setDrafts(old=>({...old,[scope]:{}}));
      } else if(review.kind==='positions'||(review.kind==='cover'&&review.imported)){
        const result=await call('album_metadata',{request:{action:review.kind==='positions'?'apply_positions':'apply_cover',
          album_id:album.id,token:review.token,write_files:write,
          positions:review.kind==='positions'?review.positions:undefined,cover_id:review.kind==='cover'?review.imported?.id:undefined}});
        count=result.count??review.ids.length;
        if(review.kind==='positions'){setPositions({});setOrder([]);}
        else setImported(undefined);
      } else {
        const selected = new Map<number,string[]>();
        if (review.kind==='cover') selected.set(review.source!.track_id,['artwork']);
        else for (const change of review.changes) {
          selected.set(change.id,[...(selected.get(change.id)??[]),change.field]);
        }
        const result = await call('album_metadata',{request:{action:'apply',album_id:album.id,token:review.token,
          rows:[...selected].map(([track_id,fields])=>({track_id,fields})),write_files:write}});
        count = result.count ?? review.ids.length;
        if (review.kind==='cover') setPath('');
        else setChoices(old=>new Set([...old].filter(key=>!review.changes.some(change=>key===changeKey(change)))));
      }
      setReview(null);
      setWrite(false);
      onSaved(review.kind==='cover' ? `Cover updated for ${album.title} (${songCount(count)}).` : `Saved details for ${songCount(count)}.`);
      invalidateDisplay();
      try { await onRefresh(); }
      catch { setNotice('Saved. Reopen the album if the library still shows old values.'); }
    } catch (error) {
      setError(errorText(error));
      setReview(null);
      setWrite(false);
      setReload(value=>value+1);
    } finally { setSaving(false); }
  }

  async function ignore(row:Row) {
    if (saving) return;
    setSaving(true);
    setError('');
    try {
      await call('metadata_workspace',{request:{action:'ignore',trackId:row.track_id,
        revision:Number(row.current.revision),token:row.candidate_token??''}});
      setChoices(old=>new Set([...old].filter(key=>!key.startsWith(`${row.track_id}:`))));
      setNotice('Suggestion moved to Ignored suggestions.');
      invalidateDisplay('album_metadata');
    } catch (error) { setError(errorText(error)); }
    finally { setSaving(false); }
  }

  const switchTab = (next:InspectorTab)=>{
    if (saving) return;
    setReview(null);
    setWrite(false);
    onTab(next);
  };

  return <section className="release-editor" aria-label={`Edit ${album.title}`}>
    <header className="album-review-heading">
      <Cover src={album.cover} title={album.title}/>
      <div><h2>{album.title}</h2><p>{[album.artist,album.year||null,songCount(album.track_ids.length)].filter(Boolean).join(' · ')}</p></div>
    </header>
    {missingTab&&!review&&<button className="primary tag-missing-action" disabled={saving}
      onClick={()=>{switchTab(missingTab);if(missingTab==='songs')onSong(0);if(missingTab==='cover')void chooseCover();}}>
      <Icon name="Plus"/>Add missing details
    </button>}
    <ManagementTabs label="Album editor" value={tab} options={[
      {id:'cover',label:'Cover'},{id:'details',label:'Details'},{id:'songs',label:'Songs'},
    ]} onChange={switchTab}/>
    {loading && !rows.length && <p className="tag-status" role="status">Loading album details…</p>}
    {loadError && <div className="tag-status" role="alert">{loadError} <button onClick={()=>setReload(value=>value+1)}>Retry comparison</button></div>}
    {comparison&&<p className="tag-comparison-status" role="status">{comparison.status==='changes_available'?`${comparison.changed_fields} suggested field changes · Nothing selected`:comparison.status==='choose_release'?'Choose the edition that matches your files.':comparison.status==='partially_matched'?`${comparison.matched_tracks??0} of ${rows.length} songs matched this release.`:comparison.status==='already_matches'?'This album already matches the checked release.':comparison.status==='lookup_failed'?'Lookup failed. Your saved values are unchanged.':comparison.status==='no_confident_match'?'No confident release match. Check recording and edition before choosing.':'This album has not been checked.'}</p>}
    {failed>0&&<details className="tag-matches"><summary>Lookup errors · {failed} {failed===1?'song':'songs'}</summary>
      {[...new Set(rows.flatMap(row=>row.lookup_error?[row.lookup_error]:[]))].map(message=><p key={message}>{message}</p>)}
    </details>}
    {!!comparison?.release_choices?.choices.length&&<details className="tag-release-choices" open={!comparison.release_choices.selected}><summary>Release choice · {comparison.release_choices.choices.length} {comparison.release_choices.choices.length===1?'edition':'editions'}</summary>{comparison.release_choices.choices.map(choice=><article key={choice.id}><div><strong>{choice.title}</strong><small>{[choice.edition,choice.date,choice.country].filter(Boolean).join(' · ')}</small><small>{choice.aligned} of {choice.total} local songs aligned · {choice.release_tracks} release tracks</small></div><button disabled={loading||saving||dirty||comparison.release_choices?.selected===choice.id} onClick={()=>{setLoading(true);setError('');void call('album_metadata',{request:{action:'select_release',album_id:album.id,release_id:choice.id}}).then(()=>{invalidateDisplay('album_metadata');setReload(v=>v+1);}).catch(e=>setError(errorText(e))).finally(()=>setLoading(false));}}>{comparison.release_choices?.selected===choice.id?'Selected':'Review this release'}</button></article>)}</details>}
    {notice && <p className="tag-status" role="status">{notice}</p>}
    {error && <p className="inline-error" role="alert">{error}</p>}
    {review && rows.length ? <>
      <section className="tag-review" aria-label="Review changes">
        <h3 ref={reviewHeading} tabIndex={-1}>{review.kind==='cover'?'Review cover update':'Review changes'}</h3>
        <p>{album.title} · {songCount(review.ids.length)}</p>
        {review.kind==='cover' ? <>
          <CoverComparison current={currentCover} proposed={{path:review.path,size:review.imported?.size??review.source?.art_resolution,local:!!review.imported}}
            lowerResolution={lowerResolution(review.imported?.size??review.source?.art_resolution)}/>
          <p className="tag-cover-note">This replaces the cover for {songCount(review.ids.length)} on {album.title}.</p>
          <details><summary>See affected songs</summary><ul>{rows.map(row=><li key={row.track_id}>{String(row.current.title)}</li>)}</ul></details>
        </> : <ChangeList changes={review.changes}/>}
        <details className="tag-advanced" onToggle={event=>{if(!event.currentTarget.open)setWrite(false);}}>
          <summary>Advanced · original file tags</summary>
          <FileWriteOption ids={review.ids} value={write} onChange={value=>{setWrite(value);setFileReady(false);}} onReady={setFileReady}/>
          {write && playing && <p role="status">Pause playback to write file tags.</p>}
        </details>
      </section>
      <footer className="metadata-action-footer">
        <button disabled={saving} onClick={()=>{setReview(null);setWrite(false);}}>Back to editing</button>
        <button className="primary" disabled={saving||loading||!!loadError||(write&&(!fileReady||playing))} onClick={()=>void save()}>
          {saving?'Saving…':write?'Save and write file tags':review.kind==='cover'?'Save cover only':'Save in musicOS'}
        </button>
      </footer>
    </> : <>
      {tab==='cover' && <section ref={dropZone} className="tag-cover" data-album-cover-drop
        onDragOver={event=>{if(event.dataTransfer.types.includes('Files'))event.preventDefault();}}
        onDrop={event=>{if(!event.dataTransfer.files.length)return;event.preventDefault();event.stopPropagation();
          if(event.dataTransfer.files.length!==1)setError('Drop one image at a time.');
          else importFile(event.dataTransfer.files[0]);
        }}>
        <div className="tag-cover-heading">
          <div><h3>Album cover</h3><p>Drop a JPEG, PNG, or WebP image here.</p></div>
          <div className="tag-cover-choices">
            <button className="primary" disabled={saving||importing} onClick={()=>void chooseCover()}><Icon name="Album"/>{importing?'Reading image…':'Choose image'}</button>
            <button disabled={!onCheck||saving||loading} onClick={onCheck}><Icon name="sync"/>Check this album</button>
          </div>
          <input type="file" ref={imageInput} hidden accept="image/jpeg,image/png,image/webp"
            onChange={event=>{importFile(event.currentTarget.files?.[0]);event.currentTarget.value='';}}/>
        </div>
        {(failed>0||unmatched>0)&&<p className="tag-cover-note">{[
          unmatched>0&&`No confident match for ${songCount(unmatched)}`,
          failed>0&&`Lookup failed for ${songCount(failed)}`,
        ].filter(Boolean).join(' · ')}</p>}
        {differentCrop&&<p className="tag-status">Different crop or proportions · compare both images before updating.</p>}
        <CoverComparison current={currentCover} proposed={imported?{path:imported.path,size:imported.size,local:true}:proposedCover&&{path:proposedCover[0],size:proposedCover[1].art_resolution}}
          lowerResolution={lowerResolution(imported?.size??proposedCover?.[1].art_resolution)} identical={identicalCover&&!artwork.length&&!imported}/>
        {imported&&<button onClick={()=>setImported(undefined)}>Remove imported image</button>}
        {!imported&&artwork.length>1 && <fieldset className="tag-cover-options"><legend>Choose a proposed cover</legend>
          {artwork.map(([cover,row],index)=><label key={cover}>
            <input name="metadata-cover" type="radio" checked={cover===path} onChange={()=>setPath(cover)}/>
            <Cover src={cover} title={`Proposed cover ${index+1}`}/>
            <span>Cover {index+1}<small>{row.art_resolution?.join(' × ') ?? String(row.current.title)}</small></span>
          </label>)}
        </fieldset>}
        {(!!artwork.length||imported) && <footer className="metadata-action-footer tag-cover-actions">
          <span>{`Applies to ${songCount(album.track_ids.length)} on ${album.title}.`}</span>
          <button className="primary" disabled={(!proposedCover&&!imported)||loading||!!loadError||importing} onClick={()=>startReview('cover')}><Icon name="Album"/>{imported?'Review cover':lowerResolution(proposedCover?.[1].art_resolution)?'Review lower-resolution cover':'Update cover only'}</button>
        </footer>}
        <MatchDetails rows={rows.filter(row=>row.candidate?.artwork_path)} disabled={saving||loading} onIgnore={row=>void ignore(row)}/>
      </section>}
      {tab==='songs' && <section className="tag-song-section">
        {track ? <div className="tag-song-heading">
          <button onClick={()=>onSong(0)}><Icon name="Back"/>All songs</button>
          <h3>{track.title}</h3>
          <div role="group" aria-label="Song editor">
            <button aria-pressed={!lyrics} onClick={()=>onLyrics(false)}>Tags</button>
            <button aria-pressed={lyrics} onClick={()=>onLyrics(true)}>Lyrics</button>
          </div>
        </div> : <>
          <div className="tag-cover-heading"><div><h3>Track order</h3>
            {duplicatePositions&&<p role="status">Some songs have the same track number on the same disc.</p>}
          </div><button disabled={saving||loading||!rows.length} onClick={renumber}>Renumber in this order</button></div>
          <div className="tag-order-list">
            {orderedRows.map((row,index)=>{
              const id=row.track_id,position=positionOf(row),title=String(row.current.title);
              return <div className="tag-order-row" key={id} data-conflict={(counts.get(positionKey(row))??0)>1||undefined}>
                <button className="tag-order-song" onClick={()=>onSong(id)}><strong>{title}</strong><small>{String(row.current.artist)}</small></button>
                {(['disc_number','track_number'] as const).map((field)=><label key={field}>{field==='disc_number'?'Disc':'Track'}
                  <input aria-label={`${fieldLabel[field]} for ${title}`} inputMode="numeric" maxLength={4} placeholder="—" value={position[field]}
                    onChange={event=>{const value=event.currentTarget.value;setPositions(old=>({...old,[id]:{...position,[field]:value}}));}}/>
                </label>)}
                <div className="tag-order-move">
                  <button disabled={index===0} aria-label={`Move ${title} earlier`} onClick={()=>movePosition(id,-1)}>↑</button>
                  <button disabled={index===orderedRows.length-1} aria-label={`Move ${title} later`} onClick={()=>movePosition(id,1)}>↓</button>
                </div>
              </div>;
            })}
          </div>
          <footer className="metadata-action-footer"><button className="primary" disabled={saving||loading||!Object.keys(positions).length}
            onClick={()=>startReview('positions')}>Review track order</button></footer>
        </>}
      </section>}
      {(tab==='details'||(tab==='songs'&&track&&!lyrics)) && !!scopedRows.length && <>
        <MetadataFields rows={scopedRows.map(row=>({...row.current,id:row.track_id}))}
          fields={scope==='album'?albumFields:metadataFields.map(([field])=>field)}
          draft={draft} onChange={update} disabled={saving}/>
        {proposals.length>0 && <details className="tag-suggestions" open>
          <summary>Suggested changes · {proposals.length}</summary>
          <ChangeList changes={proposals} choices={choices} onChoose={(group,checked)=>setChoices(old=>{
            const next=new Set(old);
            for(const change of group) checked?next.add(changeKey(change)):next.delete(changeKey(change));
            return next;
          })}/>
          <button disabled={!selected.length||loading||!!loadError||saving} onClick={()=>startReview('suggestions')}>Review selected suggestions</button>
        </details>}
        <MatchDetails rows={scopedRows} disabled={saving||loading} onIgnore={row=>void ignore(row)}/>
        <footer className="metadata-action-footer">
          <button className="primary" disabled={!Object.keys(draft).length||loading||!!loadError||saving} onClick={()=>startReview('tags')}>Review changes</button>
        </footer>
      </>}
    </>}
    {track && <KeepView active={tab==='songs'&&lyrics&&!review}>
      <LyricsEditor key={track.id} track={track} request={lyricsRequest} onDirtyChange={setLyricDirty} onBusyChange={setLyricBusy}/>
    </KeepView>}
  </section>;
}
