import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {call,errorText} from './bridge';
import {displayData,invalidateDisplay} from './displayData';
import {KeepView,useViewActive} from './ViewScope';
import {Cover,Icon,Menu} from './components';
import AlbumMetadata,{type InspectorTab} from './AlbumMetadata';
import BatchMetadata from './BatchMetadata';
import {LyricsHistory,LyricFileSync} from './LyricsEditor';
import MetadataBackups from './MetadataBackups';
import {displayMetadata,songCount} from './MetadataFields';
import type {Album,Library,MetadataHistory,MetadataStatus,Review,Task} from './types';

export type MetadataEntry = {mode:'album'; albumId:string}
  | {mode:'song'|'lyrics'; trackId:number}
  | {mode:'bulk'; ids:number[]};
type WorkspacePage = 'albums'|'bulk'|'history'|'ignored'|'profiles'|'lyric-files';
type Selection = {albumId:string; ids:number[]; trackId?:number};
type InspectorTarget = {album:Album; trackId?:number; tab:InspectorTab; lyrics?:boolean};
type Props = {
  library:Library; entry?:MetadataEntry; busy:boolean; task?:Task; playing?:boolean;
  onTask:(kind:string,albumId?:string)=>void; onRefresh:()=>Promise<void>;
};
const pageSize = 50;
const displayFields = ['title','artist','album_artist','album','genre','year','track_number','disc_number','cover_art_path'];

export default function MetadataWorkspace({library,entry,busy,task,playing=false,onTask,onRefresh}:Props) {
  const active = useViewActive();
  const list = useRef<HTMLDivElement>(null);
  const inspector = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const browserScroll = useRef(0);
  const browserFocus = useRef<string|null>(null);
  const inspectorFocus = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const handledEntry = useRef<MetadataEntry|undefined>(undefined);
  const [page,setPage] = useState<WorkspacePage>('albums');
  const [selection,setSelection] = useState<Selection|null>(null);
  const [tab,setTab] = useState<InspectorTab>('cover');
  const [lyrics,setLyrics] = useState(false);
  const [lyricsRequest,setLyricsRequest] = useState(0);
  const [showInspector,setShowInspector] = useState(false);
  const [editorKey,setEditorKey] = useState(0);
  const [dirty,setDirty] = useState(false);
  const [saving,setSaving] = useState(false);
  const [bulkDirty,setBulkDirty] = useState(false);
  const [bulkSaving,setBulkSaving] = useState(false);
  const [pendingNavigation,setPendingNavigation] = useState<(()=>void)|null>(null);
  const [bulkIds,setBulkIds] = useState<number[]>([]);
  const [bulkKey,setBulkKey] = useState(0);
  const [query,setQuery] = useState('');
  const [filter,setFilter] = useState<'all'|'suggestions'>('all');
  const [offset,setOffset] = useState(0);
  const [summary,setSummary] = useState<Set<string>|null>(null);
  const [summaryError,setSummaryError] = useState('');
  const [cancelling,setCancelling] = useState(false);
  const [cancelError,setCancelError] = useState('');
  const [message,setMessage] = useState('');
  const [epoch,setEpoch] = useState(0);

  const album = selection
    ? library.albums.find(album=>album.id===selection.albumId && selection.ids.every(id=>album.track_ids.includes(id)))
      ?? library.albums.find(album=>selection.ids.length>0 && selection.ids.every(id=>album.track_ids.includes(id)))
    : undefined;
  const track = library.tracks.find(track=>track.id===selection?.trackId);
  const displayed = useMemo(()=>{
    const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
    return library.albums.filter(album=>{
      if (filter==='suggestions' && !summary?.has(album.id)) return false;
      const text = `${album.title} ${album.artist} ${album.year||''} ${album.release_type??''}`.toLocaleLowerCase();
      return words.every(word=>text.includes(word));
    });
  },[library.albums,summary,filter,query]);
  const currentOffset = Math.min(offset,Math.max(0,Math.ceil(displayed.length/pageSize)-1)*pageSize);
  const scanning = task?.kind==='enrich' && task.running;
  useEffect(()=>{if(!scanning)setCancelling(false);},[scanning]);
  const taskResult = task?.kind==='enrich' && !task.running ? task : undefined;

  useEffect(()=>{
    if (!active) return;
    const invalidated = (event:Event)=>{
      const key = (event as CustomEvent).detail;
      if (!key || key==='album_metadata') setEpoch(epoch=>epoch+1);
    };
    window.addEventListener('musicos-display-invalidated',invalidated);
    return ()=>window.removeEventListener('musicos-display-invalidated',invalidated);
  },[active]);
  useEffect(()=>{
    if (!active || page!=='albums') return;
    let current = true;
    setSummaryError('');
    void displayData('album_metadata',{request:{action:'summary'}},true).then(result=>{
      if (current) setSummary(new Set(result.album_ids??[]));
    }).catch(error=>{if(current)setSummaryError(errorText(error));});
    return ()=>{current=false;};
  },[active,page,epoch,taskResult]);

  useLayoutEffect(()=>{
    if (!active || page!=='albums') return;
    if (inspectorFocus.current && showInspector) {
      inspectorFocus.current = false;
      inspector.current?.focus({preventScroll:true});
    }
    if (browserFocus.current!==null) {
      if(list.current)list.current.scrollTop=browserScroll.current;
      const row = list.current?.querySelector<HTMLButtonElement>(`[data-release-id="${CSS.escape(browserFocus.current)}"]`);
      (row??search.current)?.focus({preventScroll:true});
      browserFocus.current = null;
    }
  },[active,page,showInspector,editorKey]);
  useEffect(()=>{
    const node=dialog.current;
    if(pendingNavigation&&active&&node&&!node.open)node.showModal();
    return ()=>node?.close();
  },[active,pendingNavigation]);

  useEffect(()=>{
    if (!active || !entry || entry===handledEntry.current || saving || bulkSaving) return;
    handledEntry.current=entry;
    if (entry.mode==='bulk') {
      move(()=>{setBulkIds(entry.ids);setBulkKey(key=>key+1);setBulkDirty(false);setPage('bulk');},bulkDirty);
      return;
    }
    const target = entry.mode==='album'
      ? library.albums.find(album=>album.id===entry.albumId)
      : library.albums.find(album=>album.track_ids.includes(entry.trackId));
    if (!target) {
      setMessage('That release is no longer available in the library.');
      setPage('albums');
      return;
    }
    select({album:target,tab:entry.mode==='album'?'details':'songs',
      trackId:entry.mode==='album'?undefined:entry.trackId,lyrics:entry.mode==='lyrics'});
  },[active,entry,saving,bulkSaving]);

  function move(action:()=>void,guard=dirty) {
    if (saving || bulkSaving) return;
    if (guard) setPendingNavigation(()=>action);
    else action();
  }

  function select(target:InspectorTarget) {
    const same = album?.id===target.album.id && selection?.trackId===target.trackId;
    const open = ()=>{
      if (!same) {
        browserScroll.current=list.current?.clientHeight ? list.current.scrollTop : browserScroll.current;
        setSelection({albumId:target.album.id,ids:target.album.track_ids,trackId:target.trackId});
        setDirty(false);
        setEditorKey(key=>key+1);
      }
      setTab(target.tab);
      setLyrics(!!target.lyrics);
      if(target.lyrics)setLyricsRequest(request=>request+1);
      setShowInspector(true);
      setPage('albums');
      inspectorFocus.current = true;
      if (same) {
        inspector.current?.focus({preventScroll:true});
        inspectorFocus.current = false;
      }
    };
    if (same) open();
    else move(open);
  }

  useLayoutEffect(()=>{
    if (selection && !album && showInspector) {
      browserFocus.current=selection.albumId;
      setShowInspector(false);
    }
  },[selection,album,showInspector]);

  function back() {
    browserFocus.current=album?.id??'';
    setShowInspector(false);
  }

  function resetList() {
    browserScroll.current=0;
    if(list.current)list.current.scrollTop=0;
    setOffset(0);
  }

  return <section className="metadata-workspace release-workspace" data-section={page==='albums'&&showInspector&&tab==='songs'&&lyrics?'lyrics':undefined}>
    <header className="tag-workspace-heading">
      <h1>Metadata &amp; artwork</h1>
      <div className="tag-workspace-actions">
        {page!=='albums'&&<button onClick={()=>setPage('albums')}><Icon name="Back"/>Albums</button>}
        <button aria-pressed={page==='history'} onClick={()=>setPage('history')}>History</button>
        <Menu label="More metadata tools" items={[
          {label:'Bulk edit',icon:'Metadata',onSelect:()=>setPage('bulk')},
          {label:'Ignored suggestions',icon:'Delete',onSelect:()=>setPage('ignored')},
          {label:'Artist profiles',icon:'Artist',onSelect:()=>setPage('profiles')},
          {label:'Sync LyricsFiles',icon:'Lyrics',onSelect:()=>setPage('lyric-files')},
        ]}><Icon name="3dots_(nocircle)"/></Menu>
      </div>
    </header>
    {message&&<div className="tag-result"><p role="status">{message}</p><button onClick={()=>setPage('history')}>View history</button></div>}
    <KeepView active={page==='albums'}>
      {(scanning||taskResult)&&<div className="tag-check tag-check-result">
            {scanning&&<div className="tag-scan">
              <progress aria-label="Checking for suggestions" value={task.total?task.processed:undefined} max={Math.max(1,task.total)}/>
              <span role="status">{task.total?`${task.processed} of ${task.total} songs checked`:'Checking for suggestions…'}</span>
              <button disabled={cancelling} onClick={()=>{
                setCancelling(true);
                setCancelError('');
                void call('cancel_task').catch(error=>{setCancelError(errorText(error));setCancelling(false);});
              }}>{cancelling?'Cancelling…':'Cancel check'}</button>
              {cancelError&&<p role="alert">{cancelError}</p>}
            </div>}
            {taskResult&&<p role={taskResult.error?'alert':'status'}>{taskResult.cancelled?'Check cancelled. Completed suggestions are available.':taskResult.error??taskResult.message}</p>}
      </div>}
      <div className="tag-layout" data-inspector={showInspector&&!!album}>
        <aside className="tag-browser" aria-label="Albums">
          <div className="tag-check">
            <button disabled={busy||saving||bulkSaving} onClick={()=>onTask('enrich')}><Icon name="sync"/>Check library for suggestions</button>
            {!scanning&&!taskResult&&<p>Compare your library with MusicBrainz and Cover Art Archive. Changes are saved only after review.</p>}

          </div>
          <label className="tag-search">Find albums
            <input ref={search} type="search" value={query} onChange={event=>{setQuery(event.target.value);resetList();}} placeholder="Album or artist"/>
          </label>
          <div className="tag-filter" role="group" aria-label="Album filter">
            <button aria-pressed={filter==='all'} onClick={()=>{setFilter('all');resetList();}}>All albums</button>
            <button aria-pressed={filter==='suggestions'} onClick={()=>{setFilter('suggestions');resetList();}}>Has suggestions</button>
          </div>
          {summaryError&&<p role="alert">Suggestion status unavailable. <button onClick={()=>setEpoch(epoch=>epoch+1)}>Retry</button></p>}
          <div className="tag-albums" ref={list} aria-busy={filter==='suggestions'&&!summary&&!summaryError}>
            {displayed.slice(currentOffset,currentOffset+pageSize).map(release=><button key={release.id} data-release-id={release.id}
              aria-pressed={album?.id===release.id} onClick={()=>select({album:release,tab:'cover'})}>
              <Cover src={release.cover} title={release.title}/>
              <span><strong>{release.title}</strong><small>{[release.artist,release.year||null,release.release_type,songCount(release.track_ids.length)].filter(Boolean).join(' · ')}</small>
                {summary?.has(release.id)&&<span className="tag-suggestion-mark">Suggestions available</span>}
              </span>
              <Icon name="Back"/>
            </button>)}
            {!displayed.length&&<p role="status">{filter==='suggestions'&&!summary&&!summaryError?'Loading suggestions…':!library.albums.length?'Add music to manage its metadata.':query?'No albums match your search.':filter==='suggestions'?'No albums have suggestions.':'No albums available.'}</p>}
          </div>
          {displayed.length>pageSize&&<div className="tag-pages">
            <button disabled={!currentOffset} onClick={()=>{resetList();setOffset(currentOffset-pageSize);}}>Previous</button>
            <span>{currentOffset+1}–{Math.min(currentOffset+pageSize,displayed.length)} of {displayed.length}</span>
            <button disabled={currentOffset+pageSize>=displayed.length} onClick={()=>{resetList();setOffset(currentOffset+pageSize);}}>Next</button>
          </div>}
        </aside>
        <div className="tag-inspector" ref={inspector} tabIndex={-1}>
          {album ? <>
            <button className="tag-browser-back" onClick={back} disabled={saving}><Icon name="Back"/>Back to albums</button>
            <KeepView active={showInspector}><AlbumMetadata key={editorKey} album={album} track={track} tab={tab} lyrics={lyrics} lyricsRequest={lyricsRequest} epoch={epoch} playing={playing}
              onCheck={busy||saving||bulkSaving ? undefined : ()=>onTask('enrich',album.id)}
              onTab={setTab} onSong={id=>select({album,trackId:id||undefined,tab:'songs'})}
              onLyrics={setLyrics} onDirty={setDirty} onSaving={setSaving}
              onSaved={text=>setMessage(text)} onRefresh={onRefresh}/></KeepView>
          </> : <p className="tag-inspector-empty">{selection?'The release’s songs are now in separate albums. Choose an album to continue.':'Choose an album to see its cover, tags, and songs.'}</p>}
        </div>
      </div>
    </KeepView>
    <KeepView active={page==='bulk'}><BatchMetadata key={bulkKey} albums={library.albums} tracks={library.tracks} initialIds={bulkIds} onRefresh={onRefresh} onDirtyChange={setBulkDirty} onBusyChange={setBulkSaving}/></KeepView>
    <KeepView active={page==='history'}><ChangeHistory tracks={library.tracks} epoch={epoch} onRefresh={onRefresh}/></KeepView>
    {page==='ignored'&&<IgnoredSuggestions/>}
    {page==='profiles'&&<ArtistProfiles/>}
    {page==='lyric-files'&&<LyricFileSync/>}
    {pendingNavigation&&<dialog className="song-selector tag-discard" ref={dialog} aria-label="Discard metadata draft?" onCancel={event=>{event.preventDefault();setPendingNavigation(null);}}>
      <h2>Discard this draft?</h2><p>Your unsaved changes will be lost when you switch.</p>
      <div><button autoFocus onClick={()=>setPendingNavigation(null)}>Keep editing</button>
        <button onClick={()=>{const proceed=pendingNavigation;setPendingNavigation(null);proceed();}}>Discard draft</button></div>
    </dialog>}
  </section>;
}

function IgnoredSuggestions() {
  const active = useViewActive();
  const [query,setQuery] = useState('');
  const [offset,setOffset] = useState(0);
  const [items,setItems] = useState<Review[]>([]);
  const [total,setTotal] = useState(0);
  const [pending,setPending] = useState(false);
  const [error,setError] = useState('');
  const [epoch,setEpoch] = useState(0);
  useEffect(()=>{
    if (!active) return;
    let current=true;
    setPending(true);
    void displayData('metadata_workspace',{request:{action:'ignored',search:query,offset}}).then(result=>{
      if (!current) return;
      setItems(result.items??[]);
      setTotal(result.total??0);
      if(offset>0&&offset>=(result.total??0))setOffset(Math.max(0,offset-pageSize));
    }).catch(error=>{if(current)setError(errorText(error));}).finally(()=>{if(current)setPending(false);});
    return ()=>{current=false;};
  },[active,query,offset,epoch]);
  async function restore(item:Review) {
    setPending(true);
    setError('');
    try {
      await call('metadata_workspace',{request:{action:'restore',trackId:item.track_id,fingerprint:item.candidate_identity}});
      invalidateDisplay();
      setEpoch(epoch=>epoch+1);
    } catch (error) {setError(errorText(error));setPending(false);}
  }
  return <section className="tag-aux">
    <h2>Ignored suggestions</h2>
    <label>Find ignored suggestions <input type="search" value={query} onChange={event=>{setQuery(event.target.value);setOffset(0);}}/></label>
    {error&&<p role="alert">{error}</p>}
    <div className="metadata-suggestion-list" aria-busy={pending}>
      {items.map(item=><div className="suggestion-actions" key={`${item.track_id}:${item.candidate_identity}`}>
        <span><strong>{item.current_title}</strong><small>{item.current_artist}</small></span>
        <button disabled={pending} onClick={()=>void restore(item)}>Restore suggestion</button>
      </div>)}
    </div>
    {!items.length&&!pending&&<p>No ignored suggestions match.</p>}
    {total>pageSize&&<div className="tag-pages">
      <button disabled={!offset} onClick={()=>setOffset(Math.max(0,offset-pageSize))}>Previous</button>
      <span>{total} ignored</span>
      <button disabled={offset+pageSize>=total} onClick={()=>setOffset(offset+pageSize)}>Next</button>
    </div>}
  </section>;
}

function ArtistProfiles() {
  const active=useViewActive();
  const [status,setStatus]=useState<MetadataStatus|null>(null);
  const [error,setError]=useState('');
  useEffect(()=>{
    if (!active) return;
    let current=true,inFlight=false;
    async function refresh() {
      if (inFlight) return;
      inFlight=true;
      try {const status=await call('metadata_status',{});if(current)setStatus(status);}
      catch (error) {if(current)setError(errorText(error));}
      finally {inFlight=false;}
    }
    void refresh();
    const timer=setInterval(()=>void refresh(),5000);
    return ()=>{current=false;clearInterval(timer);};
  },[active]);
  return <section className="tag-aux" aria-label="Artist metadata status">
    <h2>Artist profiles</h2>
    <p role="status">{status?`${status.cached} cached · ${status.checking} checking · ${status.unavailable} unavailable`:'Checking cached profiles…'}</p>
    {error&&<p role="alert">{error}</p>}
    {!!status?.unavailable&&<button disabled={!status.retryable} onClick={()=>void call('metadata_status',{retry:true}).then(setStatus).catch(error=>setError(errorText(error)))}>
      {status.retryable?'Retry available lookups':'Waiting before retry'}
    </button>}
  </section>;
}

function ChangeHistory({tracks,epoch,onRefresh}:{tracks:Library['tracks'];epoch:number;onRefresh:()=>Promise<void>}) {
  const active=useViewActive();
  const [history,setHistory]=useState<MetadataHistory[]>([]);
  const [loading,setLoading]=useState(true);
  const [pending,setPending]=useState(false);
  const [error,setError]=useState('');
  const [message,setMessage]=useState('');
  useEffect(()=>{
    if(!active)return;
    let current=true;
    setLoading(true);
    void displayData('metadata_workspace',{request:{action:'history'}},true).then(result=>{
      if(current)setHistory(result.history??[]);
    }).catch(error=>{if(current)setError(errorText(error));}).finally(()=>{if(current)setLoading(false);});
    return ()=>{current=false;};
  },[active,epoch]);
  async function undo(id:number) {
    setPending(true);
    setError('');
    try {
      const result=await call('metadata_workspace',{request:{action:'undo',id}});
      setHistory(result.history??[]);
      setMessage('Change undone.');
      invalidateDisplay();
      await onRefresh();
    } catch (error) {setError(errorText(error));invalidateDisplay();}
    finally {setPending(false);}
  }
  return <section className="metadata-history tag-aux">
    <h2>Change history</h2>
    {error&&<p role="alert">{error}</p>}
    {message&&<p role="status">{message}</p>}
    {loading&&<p role="status">Loading history…</p>}
    {!history.length&&!loading&&<p>No metadata changes yet.</p>}
    {history.map(batch=><article key={batch.id}>
      <div><strong>{songCount(batch.changes.length)} changed</strong><small>{new Date(batch.created_at*1000).toLocaleString()}</small></div>
      <button disabled={pending||loading||batch.undone||batch.conflict} onClick={()=>void undo(batch.id)}>
        {batch.undone?'Undone':batch.conflict?'Later edits · compare below':'Undo'}
      </button>
      <details><summary>Compare changes</summary>
        {batch.changes.map(change=><div key={change.track_id}>
          <h3>{String(change.before.title)}</h3>
          {displayFields.filter(field=>displayMetadata(change.before[field])!==displayMetadata(change.after[field])).map(field=><p key={field}>
            <strong>{field==='cover_art_path'?'Cover art':field.replaceAll('_',' ')}</strong> {displayMetadata(change.before[field])} → {displayMetadata(change.after[field])}
            {change.conflict&&<> · Current {displayMetadata(change.current?.[field])}</>}
          </p>)}
        </div>)}
      </details>
    </article>)}
    <LyricsHistory tracks={tracks}/>
    <MetadataBackups/>
  </section>;
}
