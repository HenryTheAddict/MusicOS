import {useViewActive} from './ViewScope';
import {useEffect,useState} from 'react';
import {Icon} from './components';
import ThemedSwitch from './ThemedSwitch';
import {call,errorText,native} from './bridge';
import type {AnalysisStatus,Library} from './types';
export default function AudioAnalysis({library}:{library:Library}){
 const active=useViewActive();
 const [full,setFull]=useState(false);
 const [status,setStatus]=useState<AnalysisStatus>({running:false,paused:false,processed:0,total:0,features:[],errors:[]}),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 useEffect(()=>{if(!active)return;let alive=true;let pending=false;async function update(){if(pending)return;pending=true;try{const next=await call('audio_analysis',{action:'status'});if(alive)setStatus(next);}catch(e){if(alive)setError(errorText(e));}finally{pending=false;}}void update();const timer=setInterval(()=>void update(),1500);return()=>{alive=false;clearInterval(timer);};},[active]);
 async function control(action:'start'|'rescan'|'rescan_all'|'pause'|'resume'|'cancel'){setBusy(true);setError('');try{setStatus(await call('audio_analysis',{action}));window.dispatchEvent(new Event('musicos-audio-analysis-changed'));if(action.startsWith('rescan'))window.dispatchEvent(new Event('musicos-audio-rescanned'));}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 const reliable=status.features.filter(f=>(f.structure?.beat_confidence??0)>=.5).length;
 const count=status.features.length,total=library.tracks.filter(t=>t.available).length;
 return <section className="audio-analysis" aria-label="BPM and key analysis">
  <header><div><h2><Icon name="AudioBot"/>Audio analysis</h2><p>BPM, key and musical structure for DJmode. Everything stays on your Mac.</p></div><span className="analysis-count"><strong>{count}</strong> / {total}<small>songs scanned</small></span></header>
  {status.running&&<div className="analysis-progress"><div><span>{status.paused?'Paused':'Analyzing your library'}</span><span>{status.processed} / {status.total}</span></div><progress aria-label="Audio analysis progress" max={status.total||1} value={status.processed}/></div>}
  <div className="analysis-controls">{!status.running?<button className="sketch-button" disabled={busy||!native||!total} onClick={()=>void control('start')}>{count?'Analyze new songs':'Analyze library'}</button>:<><button className="sketch-button" disabled={busy} onClick={()=>void control(status.paused?'resume':'pause')}>{status.paused?'Resume analysis':'Pause analysis'}</button><button className="analysis-secondary" disabled={busy} onClick={()=>void control('cancel')}>Cancel</button></>}</div>
  {!status.running&&<div className="audio-rescan"><ThemedSwitch label="Recheck every available song" checked={full} onChange={setFull}/><button className="sketch-button" disabled={busy||!native||!total} onClick={()=>void control(full?'rescan_all':'rescan')}>Rescan audio</button><small>{full?'Refresh specifications and musical analysis for all available songs.':'Refresh changed, missing, failed, or outdated specifications and analysis.'} Valid results stay cached until replacements succeed. Files and collections stay intact.</small></div>}
  {!native&&<small>Open the desktop app to analyze local audio.</small>}
  {error&&<div className="analysis-error" role="alert"><strong>Analysis couldn’t start</strong><p>Try again. If this continues, check the details below.</p><details><summary>Technical details</summary><pre>{error}</pre></details></div>}
  {status.errors.length>0&&<div className="analysis-error"><strong>{status.errors.length} {status.errors.length===1?'song needs':'songs need'} another try</strong><p>The rest of your library can continue. Run analysis again to retry these songs.</p><details><summary>Show details</summary>{status.errors.map((e,i)=><pre key={i}>{e}</pre>)}</details></div>}
  <details className="analysis-explanation"><summary>How analysis works</summary><p>{reliable} of {count} analyzed songs have a beat grid confident enough for musical transitions. Other songs use an ending blend; an estimated BPM alone does not confirm beat alignment.</p><p>MusicOS scans full-track structure and tonal excerpts from the beginning, middle and end. Results are saved across restarts. Tempo can resolve at half or double time; uncertain keys stay unknown.</p></details>
  {count>0&&<details><summary>View BPM & key estimates</summary><div className="analysis-results">{status.features.map(f=><div key={f.track_id}><strong>{library.tracks.find(t=>t.id===f.track_id)?.title??'Unavailable song'}</strong><span>{f.bpm?`${Math.round(f.bpm)} BPM${f.bpm_confidence<0.5?' (uncertain)':''}`:'Tempo unknown'} · {f.key&&f.key_confidence>=0.5?f.key:'Key uncertain'}</span></div>)}</div></details>}
 </section>;
}
