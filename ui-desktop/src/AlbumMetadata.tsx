import {useEffect,useRef,useState} from 'react';
import {call,errorText} from './bridge';
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
type Review = {kind:'cover'; token:string; source:Row; path:string; ids:number[]}
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

export default function AlbumMetadata({album,track,tab,lyrics,lyricsRequest=0,epoch,playing,onTab,onSong,onLyrics,onDirty,onSaving,onSaved,onRefresh}: {
  album:Album; track?:Track; tab:InspectorTab; lyrics:boolean; lyricsRequest?:number; epoch:number; playing:boolean;
  onTab:(tab:InspectorTab)=>void; onSong:(id:number)=>void; onLyrics:(show:boolean)=>void;
  onDirty:(dirty:boolean)=>void; onSaving:(saving:boolean)=>void;
  onSaved:(message:string)=>void; onRefresh:()=>Promise<void>;
}) {
  const active = useViewActive();
  const [comparison,setComparison] = useState<AlbumReview|null>(null);
  const [drafts,setDrafts] = useState<{album:MetadataDraft;song:MetadataDraft}>({album:{},song:{}});
  const [choices,setChoices] = useState<Set<string>>(new Set());
  const [path,setPath] = useState('');
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
  const rows = comparison?.album_id===album.id ? comparison.rows : [];
  const scope = tab==='songs' && track ? 'song' : 'album';
  const scopedRows = scope==='song' ? rows.filter(row=>row.track_id===track?.id) : rows;
  const draft = drafts[scope];
  const proposals = suggestedChanges(scopedRows);
  const selected = proposals.filter(change=>choices.has(changeKey(change)));
  const artwork = [...new Map(rows.flatMap(row => {
    const cover = row.candidate?.artwork_path;
    return typeof cover==='string' && cover!==comparison?.cover ? [[cover,row] as const] : [];
  })).entries()];
  const proposedCover = artwork.find(([cover])=>cover===path) ?? (artwork.length===1 ? artwork[0] : undefined);
  const dirty = Object.values(drafts).some(values=>Object.keys(values).length>0) || choices.size>0 || !!path || lyricDirty || !!review;
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
    }).catch(error=>{if(current)setLoadError(errorText(error));}).finally(()=>{if(current)setLoading(false);});
    return ()=>{current=false;};
  },[active,album.id,epoch,reload]);

  useEffect(()=>{
    setReview(null);
    setWrite(false);
  },[tab,lyrics]);
  useEffect(()=>{if(review)reviewHeading.current?.focus({preventScroll:true});},[review]);

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
    if (kind==='cover' && proposedCover) {
      setReview({kind,token:comparison.token,path:proposedCover[0],source:proposedCover[1],ids:rows.map(row=>row.track_id)});
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
      } else {
        const selected = new Map<number,string[]>();
        if (review.kind==='cover') selected.set(review.source.track_id,['artwork']);
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
    <ManagementTabs label="Album editor" value={tab} options={[
      {id:'cover',label:'Cover'},{id:'details',label:'Details'},{id:'songs',label:'Songs'},
    ]} onChange={switchTab}/>
    {loading && !rows.length && <p className="tag-status" role="status">Loading album details…</p>}
    {loadError && <div className="tag-status" role="alert">{loadError} <button onClick={()=>setReload(value=>value+1)}>Retry comparison</button></div>}
    {notice && <p className="tag-status" role="status">{notice}</p>}
    {error && <p className="inline-error" role="alert">{error}</p>}
    {review && rows.length ? <>
      <section className="tag-review" aria-label="Review changes">
        <h3 ref={reviewHeading} tabIndex={-1}>{review.kind==='cover'?'Review cover update':'Review changes'}</h3>
        <p>{album.title} · {songCount(review.ids.length)}</p>
        {review.kind==='cover' ? <>
          <div className="tag-cover-comparison">
            <figure><Cover src={comparison?.cover} title="Current cover"/><figcaption>Current</figcaption></figure>
            <figure><Cover src={review.path} title="New cover"/><figcaption>New cover</figcaption></figure>
          </div>
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
          {saving?'Saving…':write?'Save and write file tags':'Save in musicOS'}
        </button>
      </footer>
    </> : <>
      {tab==='cover' && <section className="tag-cover">
        <div className="tag-cover-comparison">
          <figure><Cover src={comparison?.cover ?? album.cover} title="Current cover"/><figcaption>Current{comparison?.resolution && ` · ${comparison.resolution.join(' × ')}`}</figcaption></figure>
          {proposedCover && <figure><Cover src={proposedCover[0]} title="Proposed cover"/><figcaption>Proposed{proposedCover[1].art_resolution && ` · ${proposedCover[1].art_resolution.join(' × ')}`}{proposedCover[1].candidate?.release_mbid ? ' · Cover Art Archive' : ''}</figcaption></figure>}
        </div>
        {artwork.length>1 && <fieldset className="tag-cover-options"><legend>Choose a proposed cover</legend>
          {artwork.map(([cover,row],index)=><label key={cover}>
            <input name="metadata-cover" type="radio" checked={cover===path} onChange={()=>setPath(cover)}/>
            <Cover src={cover} title={`Proposed cover ${index+1}`}/>
            <span>Cover {index+1}<small>{row.art_resolution?.join(' × ') ?? String(row.current.title)}</small></span>
          </label>)}
        </fieldset>}
        {!artwork.length && !loading && !loadError && <p>No cover suggestions for this album.</p>}
        {!!artwork.length && <footer className="metadata-action-footer">
          <button className="primary" disabled={!proposedCover||loading||!!loadError} onClick={()=>startReview('cover')}><Icon name="Album"/>Update cover only</button>
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
        </div> : <div className="tag-songs">
          {rows.map(row=><button key={row.track_id} onClick={()=>onSong(row.track_id)}>
            <small>{Number(row.current.disc_number)>0?`Disc ${row.current.disc_number} · `:''}{Number(row.current.track_number)>0?`Track ${row.current.track_number}`:''}</small>
            <strong>{String(row.current.title)}</strong><span>{String(row.current.artist)}</span>
          </button>)}
        </div>}
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
