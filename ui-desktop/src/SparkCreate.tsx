import {useScrollEdges} from './useScrollEdges';
import './sparklistPolish.css';
import{useViewActive}from'./ViewScope';
import{animateOwned}from'./presentationMotion';
import {resolveArtworkTheme} from './artworkTheme';
import {sparkNames,moodHue,hexHue} from './sparkSuggestions';
import SparkLength from './SparkLength';
import SparkSetup from './SparkSetup';
import SongSelector from './SongSelector';
import {collectionCover,suggestedSparkCover} from './CollectionDesign';
import {encodeVibe,parseVibe} from './vibeArtwork';
import {useEffect,useState,useRef,useLayoutEffect,useMemo} from 'react';
import {call,errorText} from './bridge';
import {Cover,Icon,Menu,duration,collectionDuration} from './components';
import {CoverChoice} from './CollectionDesign';
import type {Library,SavedCollection,SparkGeneration,SparkState,Preferences,Track,SparkRefinement,SparkPreview} from './types';
const moods=['Chilled','Energetic','Melancholic','Bright','Focused','Dreamy'];
export default function SparkCreate({library,items,onChange,onOpen,onClose,onPlay,onToggle,playingId,isPlaying,onSetup,preferences,onPreferences,initialSeed}:{initialSeed?:number;preferences?:Preferences;onPreferences?:(p:Preferences)=>void;library:Library;items:SavedCollection[];onChange:(items:SavedCollection[])=>void;onOpen:(item:SavedCollection)=>void;onClose:()=>void;onPlay:(ids:number[])=>void|Promise<void>;onToggle?:()=>Promise<void>;playingId?:number;isPlaying?:boolean;onSetup:()=>void}){
 const [refinement,setRefinement]=useState<SparkRefinement>({seed_ids:initialSeed===undefined?[]:[initialSeed],moods:[],variety:50,discovery:50,kept:[],excluded_ids:[],more_like:[]});
 const [undo,setUndo]=useState<SparkPreview[]>([]);
 const dragging=useRef<number|null>(null);
 const [targetCount,setTargetCount]=useState(25),[artworkEdited,setArtworkEdited]=useState(false);
 const [workLabel,setWorkLabel]=useState('Working…');
 const [previewStarted,setPreviewStarted]=useState(false);
 const [stage,setStage]=useState<1|2|3>(1);const [choosingSeed,setChoosingSeed]=useState(false);
 const [seed,setSeed]=useState<number|null>(initialSeed??null),[mood,setMood]=useState<string|null>(null),[state,setState]=useState<SparkState|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[name,setName]=useState(''),[choice,setChoice]=useState('auto/1'),[feedback,setFeedback]=useState<number[]>([]),[variant,setVariant]=useState(0);
 const active=useViewActive(),[statusEpoch,setStatusEpoch]=useState(0),root=useRef<HTMLElement>(null);
 useEffect(()=>{if(!active)return;let alive=true,pending=false,timer=0;const checkingUntil=performance.now()+60000;async function refresh(){if(pending)return;pending=true;try{const s=await call('sparklist',{request:{action:'status'}});if(alive){setState(old=>({...s,preview:s.ready?old?.preview??null:null,generated_artwork:old?.generated_artwork??null}));if(s.installation?.running||s.analysis?.running&&!s.analysis.paused||!s.ready&&s.beta&&performance.now()<checkingUntil)timer=window.setTimeout(refresh,1500);}}catch(e){if(alive)setError(errorText(e));}finally{pending=false;}}void refresh();const changed=()=>{if(!pending)void refresh();};window.addEventListener('musicos-audio-rescanned',changed);window.addEventListener('musicos-spark-ready',changed);return()=>{alive=false;clearTimeout(timer);window.removeEventListener('musicos-audio-rescanned',changed);window.removeEventListener('musicos-spark-ready',changed);};},[active,statusEpoch]);
 useLayoutEffect(()=>{const node=root.current?.querySelector<HTMLElement>(stage===1?'.spark-start-body':'.spark-preview:not([hidden])');const dialog=root.current?.closest('dialog');if(dialog)dialog.scrollTop=0;if(stage>1)root.current?.querySelector<HTMLElement>(stage===3?'.spark-name-field input':'.spark-stage-title')?.focus({preventScroll:true});if(node)return animateOwned(node,'spark-step',[{opacity:.45,transform:'translateY(6px)'},{opacity:1,transform:'none'}],180);},[stage]);
 const autoPreviewed=useRef(false);
 useEffect(()=>{if(initialSeed===undefined||autoPreviewed.current||busy||!state?.ready||state.analyzed<2||!state.analyzed_ids.includes(initialSeed))return;autoPreviewed.current=true;void preview();},[initialSeed,state?.ready,state?.analyzed,busy]);
 const generation:SparkGeneration={seed,mood,less_like:feedback,variant,target_count:targetCount,refinement};
 const lookup=useMemo(()=>new Map(library.tracks.map(t=>[t.id,t])),[library.tracks]);
 const selected=seed===null?undefined:lookup.get(seed);
 const previewTracks=useMemo(()=>(state?.preview?.track_ids??[]).map(id=>lookup.get(id)).filter((t):t is Track=>!!t),[state?.preview,lookup]);
 const previewSeconds=previewTracks.reduce((sum,t)=>sum+t.duration,0),previewArtists=new Set(previewTracks.map(t=>t.artist)).size,previewAlbums=new Set(previewTracks.map(t=>`${t.artist}:${t.album}`)).size;
 const previewCover=useMemo(()=>collectionCover(choice),[choice]);
 const previewPlaying=previewStarted&&isPlaying&&playingId!==undefined&&!!state?.preview?.track_ids.includes(playingId);
 const enough=(state?.analyzed??0)>=2;
 const review=useRef<HTMLOListElement>(null),startBody=useRef<HTMLDivElement>(null),saveForm=useRef<HTMLFormElement>(null);useScrollEdges(review,active&&stage===2,state?.preview);useScrollEdges(startBody,active&&stage===1,state?.ready);useScrollEdges(saveForm,active&&stage===3,state?.preview);
 const hue=mood?moodHue[mood]??255:parseVibe(choice)?.hue??255;
 const suggestions=sparkNames(selected?.title,mood,parseVibe(choice)?.hue);
 function naming(){setStage(3);}
 async function listen(){if(busy||!state?.preview)return;setWorkLabel("Opening the preview…");setBusy(true);setError('');try{if(previewStarted&&playingId!==undefined&&state.preview.track_ids.includes(playingId)&&onToggle)await onToggle();else {await onPlay(state.preview.track_ids);setPreviewStarted(true);}}catch(e){setError(`Could not play this preview. ${errorText(e)}`);}finally{setBusy(false);}}
 async function seedColors(){if(!selected)return;setWorkLabel('Choosing cover colors…');setArtworkEdited(true);setBusy(true);setError('');try{const palette=await resolveArtworkTheme([selected.cover_art_path],selected.id),art=parseVibe(choice);if(!palette||!art)throw new Error('Cover colors are unavailable. Try sound or mood colors.');const hue=hexHue(palette.palette.accent);if(hue!==null)setChoice(encodeVibe({...art,version:2,hue}));}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 function artworkColors(source:'sound'|'mood'){setArtworkEdited(true);const art=parseVibe(choice);if(source==='sound'&&state?.generated_artwork)setChoice(state.generated_artwork);else if(art&&mood)setChoice(encodeVibe({...art,version:2,hue:moodHue[mood]??art.hue}));}
 async function analyzeLibrary(){setWorkLabel('Preparing songs…');setBusy(true);setError('');try{await call('audio_analysis',{action:'start'});setStatusEpoch(n=>n+1);}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 function invalidate(){setState(s=>s?{...s,preview:null}:s);setPreviewStarted(false);setUndo([]);}
 function remember(){if(state?.preview)setUndo(old=>[...old.slice(-19),structuredClone(state.preview!)]);}
 function accept(next:SparkState){
  setState(next);
  if(next.preview){const config=next.preview.configuration;setRefinement(config.refinement??refinement);setFeedback(config.less_like);setVariant(config.variant);}
  setPreviewStarted(false);
 }
 function editPreview(ids:number[],next:SparkRefinement){
  if(!state?.preview)return;
  remember();
  next={...next,kept:next.kept.filter(k=>ids.includes(k.track_id)).map(k=>({...k,position:ids.indexOf(k.track_id)}))};
  setRefinement(next);
  setState({...state,preview:{...state.preview,track_ids:ids,configuration:{...generation,refinement:next}}});
 }
 function restore(){
  const previous=undo.at(-1);if(!previous||!state)return;
  setUndo(old=>old.slice(0,-1));setState({...state,preview:previous});
  setRefinement(previous.configuration.refinement??refinement);setFeedback(previous.configuration.less_like);setVariant(previous.configuration.variant);setPreviewStarted(false);
 }
 function moveSong(from:number,to:number){
  if(!state?.preview||from===to||to<0||to>=previewTracks.length)return;
  const ids=[...state.preview.track_ids];const [id]=ids.splice(from,1);ids.splice(to,0,id);editPreview(ids,refinement);
 }
 async function replaceSong(id:number){
  if(busy||!state?.preview)return;
  setBusy(true);setWorkLabel('Replacing this song…');setError('');
  try{const next=await call('sparklist',{request:{action:'replace',generation,preview:state.preview,track_id:id}});remember();accept(next);}
  catch(e){setError(errorText(e));}finally{setBusy(false);}
 }
 async function moreLike(id:number){
  const next={...refinement,more_like:[...new Set([...refinement.more_like,id])].slice(-32)};
  await preview(true,undefined,next);
 }
 async function choose(){if(busy)return;setWorkLabel("Choosing a starting song…");setBusy(true);setError('');try{const next=await call('sparklist',{request:{action:'choose'}});setSeed(next.chosen_id);setFeedback([]);setVariant(0);if(next.chosen_id!==null){const generated=await call('sparklist',{request:{action:'preview',generation:{seed:next.chosen_id,mood,less_like:[],variant:0,target_count:targetCount,refinement:{...refinement,seed_ids:[next.chosen_id],kept:[]}}}});setRefinement(generated.preview?.configuration.refinement??{...refinement,seed_ids:[next.chosen_id!]});setState(generated);setPreviewStarted(false);if(generated.generated_artwork&&!artworkEdited)setChoice(suggestedSparkCover(generated.generated_artwork));setStage(2);}else setState(next);}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 async function preview(regenerate=false,lessLike?:number,options=refinement){if(busy)return;setWorkLabel(lessLike!==undefined?'Refining your mix…':regenerate?'Finding another sequence…':'Finding songs with a similar sound…');setBusy(true);setError('');const nextFeedback=lessLike===undefined?feedback:[...new Set([...feedback,lessLike])];const nextVariant=regenerate?variant+1:variant;try{const next=await call('sparklist',{request:{action:'preview',generation:{...generation,less_like:nextFeedback,variant:nextVariant,refinement:{...options,kept:options.kept.filter(k=>k.track_id!==lessLike)}}}});remember();accept(next);if(next.generated_artwork&&!artworkEdited)setChoice(suggestedSparkCover(next.generated_artwork));setStage(2);setFeedback(nextFeedback);setVariant(nextVariant);}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 async function save(){if(busy)return;setWorkLabel("Saving your Sparklist…");setBusy(true);setError('');try{const next=await call('sparklist',{request:{action:'save',name,artwork_choice:choice,generation,preview:state?.preview??undefined}});onChange(next.items);const added=next.items.find(c=>!items.some(old=>old.id===c.id));if(added)onOpen(added);onClose();}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <section ref={root} className="spark-create-content" style={{'--spark-color':`hsl(${hue} 70% 60%)`,'--spark-color-secondary':`hsl(${(hue+60)%360} 68% 65%)`} as import('react').CSSProperties} data-stage={stage} aria-busy={busy}>
 <header className="spark-composer-heading"><span className="spark-composer-mark"><Icon name="Sparklist"/></span><div><h2 className="spark-stage-title" tabIndex={-1}>{stage===1?'Create Sparklist':stage===2?'Preview Sparklist':'Save Sparklist'}</h2></div><button className="spark-icon-action" aria-label="Close Sparklist creator" onClick={onClose}><Icon name="controlClose"/></button></header>
 <nav className="spark-stages" aria-label="Sparklist creation steps">{['Choose','Refine','Save'].map((label,i)=><button key={label} aria-label={`Step ${i+1}: ${label}`} disabled={busy||(i>0&&!state?.preview)} aria-current={stage===i+1?'step':undefined} onClick={()=>setStage((i+1) as 1|2|3)}><span>{i+1}</span>{label}</button>)}</nav>
 <div className="spark-build-status" data-working={busy} role="status"><Icon name={busy?'sync':'Sparklist'}/><div><strong>{busy?workLabel:state?.ready?`${state.analyzed} songs ready`:state&&!state.beta?'Sparklists are off':'Preparing Sparklists'}</strong>{state?.analysis?.running&&<small>{state.analysis.paused?'Analysis paused':`${state.analysis.processed} of ${state.analysis.total} songs prepared`}</small>}</div>{state?.analysis?.running&&<progress aria-label="Song preparation" max={state.analysis.total||1} value={state.analysis.processed}/>}</div>
 <div ref={startBody} className="spark-start-body scroll-edges" hidden={stage!==1}>
  <aside className="spark-start-art" aria-hidden="true"><img src={previewCover} alt=""/><strong>{mood??"New Sparklist"}</strong><span>{targetCount} songs</span></aside><div className="spark-start-controls"><section className="spark-source-group"><div className="spark-section-label"><h3>Starting song</h3><span>Optional</span></div><div className="spark-seed-row"><button className="spark-seed-toggle" aria-label={selected?'Add starting song':'Select song'} aria-haspopup="dialog" aria-expanded={choosingSeed} disabled={busy||refinement.seed_ids.length>=3} onClick={()=>setChoosingSeed(true)}>{selected?<><Cover songArtwork trackId={selected.id} src={selected.cover_art_path} title={selected.title}/><span><small>Starting from</small><strong>{selected.title}</strong><small>{selected.artist}</small></span></>:<><span className="spark-seed-placeholder"><Icon name="Song"/></span><span><strong>Select song</strong><small>Choose from your library</small></span></>}<Icon name="search"/></button>{selected&&<button className="spark-icon-action" aria-label="Clear starting song" disabled={busy} onClick={()=>{setSeed(null);setRefinement({...refinement,seed_ids:[],kept:[]});invalidate();}}><Icon name="controlClose"/></button>}</div></section>
  {choosingSeed&&<SongSelector tracks={library.tracks} selected={seed} onClose={()=>setChoosingSeed(false)} onSelect={id=>{
   if(refinement.seed_ids.includes(id)){setChoosingSeed(false);return;}
   const ids=refinement.seed_ids.length? [...refinement.seed_ids,id].slice(0,3):[id];
   setSeed(ids[0]);setRefinement({...refinement,seed_ids:ids,kept:[]});invalidate();setChoosingSeed(false);
  }}/>}
  {refinement.seed_ids.length>0&&<div className="spark-seed-chips">{refinement.seed_ids.map(id=><button key={id} disabled={busy} onClick={()=>{
   const ids=refinement.seed_ids.filter(value=>value!==id);setSeed(ids[0]??null);setRefinement({...refinement,seed_ids:ids});invalidate();
  }} aria-label={`Remove starting song ${lookup.get(id)?.title??id}`}><Cover src={lookup.get(id)?.cover_art_path} title=""/><span>{lookup.get(id)?.title}</span><Icon name="controlClose"/></button>)}</div>}
  <fieldset className="spark-moods"><legend>Mood</legend>{moods.map(value=><button key={value} style={{'--mood-color':`hsl(${moodHue[value]??255} 65% 60%)`} as import('react').CSSProperties} disabled={busy} aria-pressed={refinement.moods.includes(value)} onClick={()=>{
   const next=refinement.moods.includes(value)?refinement.moods.filter(m=>m!==value):[...refinement.moods,value].slice(-2);
   setMood(next[0]??null);setRefinement({...refinement,moods:next});invalidate();
  }}><span className="spark-mood-dot" aria-hidden="true"/>{value}</button>)}</fieldset>
  <SparkLength value={targetCount} disabled={busy} onChange={value=>{setTargetCount(value);setState(s=>s?{...s,preview:null}:s);setPreviewStarted(false);}}/>
  <div className="spark-refinement-controls"><label>Variety <output>{refinement.variety}%</output><input type="range" aria-label="Variety" min="0" max="100" value={refinement.variety} disabled={busy} onChange={e=>{setRefinement({...refinement,variety:Number(e.target.value)});invalidate();}}/></label>
  <label>Familiar ↔ Discovery <output>{refinement.discovery}%</output><input type="range" aria-label="Familiar to Discovery" min="0" max="100" value={refinement.discovery} disabled={busy} onChange={e=>{setRefinement({...refinement,discovery:Number(e.target.value)});invalidate();}}/></label></div>
  {state&&!state.ready&&(preferences&&onPreferences?<SparkSetup enabled={!!preferences.sparklist_beta} onChange={sparklist_beta=>onPreferences({...preferences,sparklist_beta})}/>:<div className="spark-availability"><p>Prepare your music to make Sparklists.</p><button onClick={onSetup}>Open model setup</button></div>)}
  {state?.ready&&!enough&&<div className="spark-model-note"><p>Prepare at least two songs to make a mix. {state.analyzed} ready so far.</p><button disabled={busy} onClick={()=>void analyzeLibrary()}>Analyze library</button></div>}
  {state?.ready&&seed!==null&&!state.analyzed_ids.includes(seed)&&<div className="spark-model-note"><p>This starting song needs preparation.</p><button disabled={busy} onClick={()=>{setWorkLabel('Preparing your starting song…');setBusy(true);void call('audio_analysis',{action:'analyze_seed',ids:[seed!]}).then(()=>setError('')).catch(e=>setError(errorText(e))).finally(()=>setBusy(false));}}>Analyze starting song</button><small>{state.analysis?.paused?'Analysis is paused.':state.analysis?.running?`${state.analysis.processed} of ${state.analysis.total} prepared`:'Your starting song will be prioritized.'}</small>{state.analysis?.paused&&<button onClick={()=>void call('audio_analysis',{action:'resume'}).then(()=>setStatusEpoch(n=>n+1)).catch(e=>setError(errorText(e)))}>Resume analysis</button>}</div>}
 </div></div>
 {stage===1&&<footer className="spark-step-footer spark-create-primary"><button className="sketch-button" disabled={busy||!state?.ready||!enough||(seed===null&&!mood)||refinement.seed_ids.some(id=>!state.analyzed_ids.includes(id))} onClick={()=>void preview()}><Icon name="Sparklist"/>Preview Sparklist</button><button className="spark-soft-action" disabled={busy||!state?.ready||!enough} onClick={()=>void choose()}><Icon name="Sparklist"/>Choose for me</button></footer>}
 {state?.preview&&<section hidden={stage===1} className="spark-preview" aria-label="Sparklist preview">
  <div className="spark-mix-layout"><aside className="spark-mix-summary"><div className="spark-generated-cover">{previewCover&&<img src={previewCover} alt="Sparklist cover artwork"/>}</div><h3>{selected?`More like ${selected.title}`:mood?`${mood} listening`:'Sparklist preview'}</h3><p>{previewTracks.length} songs · {collectionDuration(previewSeconds)}</p><small>{previewArtists} artists · {previewAlbums} albums</small>{stage===2&&<button aria-label={previewPlaying?'Pause preview':'Listen to preview'} disabled={busy} className="spark-preview-play" onClick={()=>void listen()}><Icon name={previewPlaying?'pause':'play'}/><span>{previewPlaying?'Pause preview':'Listen to preview'}</span></button>}</aside>
  <div className="spark-mix-workspace"><div hidden={stage!==2} className="spark-track-review"><header><h3>In this mix</h3><button className="spark-soft-action" disabled={busy||!undo.length} onClick={restore}>Undo</button><button className="spark-soft-action" disabled={busy} onClick={()=>void preview(true)}><Icon name="sync"/>Regenerate</button></header>{state.preview.track_ids.length<targetCount&&<p className="editor-hint" role="status">{state.preview.track_ids.length} of {targetCount} requested songs available. Prepare more music or clear feedback for a longer mix.</p>}{state.preview.analyzed<state.preview.available&&<details className="spark-preview-coverage"><summary>Some songs are still preparing</summary><p>This mix uses {state.preview.analyzed} of {state.preview.available} available songs.</p></details>}<ol ref={review} className="spark-preview-list scroll-edges">{previewTracks.map((track,index)=>{
 const kept=refinement.kept.some(k=>k.track_id===track.id);
 return <li key={track.id} draggable={!busy} onDragStart={event=>{dragging.current=index;event.dataTransfer.effectAllowed='move';event.dataTransfer.setData('text/plain',String(index));}} onDragEnd={()=>{dragging.current=null;}} onDragOver={event=>{if(dragging.current!==null)event.preventDefault();}} onDrop={event=>{event.preventDefault();if(dragging.current!==null)moveSong(dragging.current,index);dragging.current=null;}}>
  <span className="spark-track-number">{index+1}</span><Cover src={track.cover_art_path} title={track.album}/>
  <span className="spark-track-copy"><strong>{track.title}</strong><small>{track.artist}</small></span><span className="spark-track-time">{duration(track.duration)}</span>
  <button className="spark-keep" aria-pressed={kept} disabled={busy} onClick={()=>editPreview(state.preview!.track_ids,{...refinement,kept:kept?refinement.kept.filter(k=>k.track_id!==track.id):[...refinement.kept,{track_id:track.id,position:index}]})}>{kept?'Kept':'Keep'}</button>
  <Menu label={`Refine ${track.title}`} items={[
   {label:'Replace song',icon:'sync',disabled:busy,onSelect:()=>void replaceSong(track.id)},
   {label:'More like this',icon:'Plus',disabled:busy,onSelect:()=>void moreLike(track.id)},
   {label:'Less like this',icon:'Song',disabled:busy,onSelect:()=>void preview(true,track.id)},
   {label:'Move up',icon:'Back',disabled:busy||index===0,onSelect:()=>moveSong(index,index-1)},
   {label:'Move down',icon:'Back',disabled:busy||index===previewTracks.length-1,onSelect:()=>moveSong(index,index+1)},
   {label:'Remove song',icon:'controlClose',disabled:busy,onSelect:()=>editPreview(state.preview!.track_ids.filter(id=>id!==track.id),{...refinement,excluded_ids:[...refinement.excluded_ids,track.id]})},
  ]}><Icon name="3dots_(nocircle)"/></Menu>
 </li>;
})}</ol></div>
   <form ref={saveForm} className="scroll-edges" id="spark-save-form" hidden={stage!==3} onSubmit={e=>{e.preventDefault();void save();}}><label className="spark-name-field">Name<input value={name} required maxLength={120} onChange={e=>setName(e.target.value)} placeholder="Name your Sparklist"/></label><div className="spark-name-suggestions" role="group" aria-label="Suggested names">{suggestions.map(suggestion=><button key={suggestion} type="button" onClick={()=>setName(suggestion)}>{suggestion}</button>)}</div><h3>Cover artwork</h3><CoverChoice spark value={choice} onChange={value=>{setArtworkEdited(true);setChoice(value);}}/><details className="spark-artwork-details"><summary>Fine-tune cover colors</summary><button type="button" disabled={busy} onClick={()=>{setArtworkEdited(true);const art=parseVibe(choice);if(art)setChoice(encodeVibe({...art,version:2,variant:(art.variant+1)>>>0,hue:(art.hue+17)%360}));else setChoice(`auto/${1+(Number(choice.split('/')[1]??0)%9)}`);}}><Icon name="sync"/>Regenerate artwork</button>{parseVibe(choice)&&<div className="spark-artwork-sources" role="group" aria-label="Artwork colors"><button type="button" disabled={busy} onClick={()=>artworkColors('sound')}>Sound colors</button><button type="button" disabled={busy||!mood} onClick={()=>artworkColors('mood')}>Mood colors</button><button type="button" disabled={busy||!selected} onClick={()=>void seedColors()}>Seed cover colors</button></div>}{state.generated_artwork&&<button type="button" onClick={()=>setChoice(state.generated_artwork!)}>Use sound-derived artwork</button>}</details></form>
  </div></div>
  <footer className="spark-step-footer"><button className="spark-soft-action" disabled={busy} onClick={()=>setStage(stage===2?1:2)}><Icon name="Back"/>{stage===2?'Edit selection':'Back to preview'}</button>{stage===2?<button key="next" type="button" disabled={busy} className="sketch-button" onClick={naming}>Name &amp; save<Icon name="Enter"/></button>:<button key="save" type="submit" form="spark-save-form" className="sketch-button" disabled={busy||!name.trim()||!state.preview?.track_ids.length}><Icon name="Sparklist"/>{busy?'Saving…':'Save Sparklist'}</button>}</footer>
 </section>}
 {error&&<p className="spark-inline-error" role="alert">{error}</p>}
 </section>;
}
