import {useEffect,useRef,useState} from 'react';
import {useViewActive} from './ViewScope';
import {call,errorText} from './bridge';
export type LyricWord={text:string;start_ms:number;end_ms?:number};
export type LyricLine={text:string;start_ms:number;end_ms?:number;words?:LyricWord[]};
export type LyricDocument={version:'1.0';metadata:{title:string;artist:string;album:string;duration_ms?:number;offset_ms:number;language?:string;instrumental:boolean};lines:LyricLine[];plain:string};
export type LyricCandidate={id:number;title:string;artist:string;album:string;duration:number;document:LyricDocument};
export type LyricResponse={before?:LyricDocument|null;after?:LyricDocument|null;track_id:number;revision:number;document:LyricDocument|null;source:{kind:string;label:string;url:string|null;provider_id:number|null}|null;state:string;message?:string;candidates:LyricCandidate[];history?:{id:number;track_id:number;at:number;revision:number;undone:boolean}[]};
export type LyricRequest={action:string;trackId?:number;revision?:number;document?:LyricDocument;candidateId?:number;text?:string;format?:'lyricsfile'|'lrc'|'plain';path?:string;overwrite?:boolean;historyId?:number;retry?:boolean;writeFiles?:boolean};
const pending=new Map<string,Promise<LyricResponse>>();
export function lyricCall(request:LyricRequest){const key=JSON.stringify(request);if(['load','lookup','search'].includes(request.action)){const old=pending.get(key);if(old)return old;const job=call('lyrics',{request}).finally(()=>pending.delete(key));pending.set(key,job);return job;}return call('lyrics',{request});}
export function useLyrics(trackId:number|undefined,active:boolean,online=true){const visible=useViewActive();active=active&&visible;const [result,setResult]=useState<LyricResponse|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');const generation=useRef(0),[epoch,setEpoch]=useState(0);
 useEffect(()=>{if(!active)return;const changed=(event:Event)=>{if((event as CustomEvent<number>).detail===trackId)setEpoch(v=>v+1);};window.addEventListener('musicos-lyrics-changed',changed);return()=>window.removeEventListener('musicos-lyrics-changed',changed);},[active,trackId]);
 useEffect(()=>{const revision=++generation.current;setResult(null);setError('');if(!active||!trackId){setBusy(false);return;}setBusy(true);void lyricCall({action:'load',trackId}).then(async r=>{if(revision!==generation.current)return;r=r.document||!online||r.state!=='missing'?r:await lyricCall({action:'lookup',trackId});if(revision===generation.current)setResult(r);}).catch(e=>{if(revision===generation.current)setError(errorText(e));}).finally(()=>{if(revision===generation.current)setBusy(false);});return()=>{generation.current++;};},[trackId,active,online,epoch]);
 async function find(search=false){if(!trackId)return;const revision=++generation.current;setBusy(true);setError('');try{const r=await lyricCall({action:search?'search':'lookup',trackId,retry:true});if(revision===generation.current)setResult(r);}catch(e){if(revision===generation.current)setError(errorText(e));}finally{if(revision===generation.current)setBusy(false);}}
 async function select(id:number){if(!result||!trackId)return;const revision=++generation.current;setBusy(true);try{const selected=await lyricCall({action:'select',trackId,revision:result.revision,candidateId:id});if(revision!==generation.current)return;setResult(selected);window.dispatchEvent(new CustomEvent('musicos-lyrics-changed',{detail:trackId}));}catch(e){if(revision===generation.current)setError(errorText(e));}finally{if(revision===generation.current)setBusy(false);}}
 return {result,busy,error,find,select};
}
export function lyricIndex(doc:LyricDocument,position:number){const ms=position*1000-doc.metadata.offset_ms;let low=0,high=doc.lines.length;while(low<high){const mid=(low+high)>>>1;if(doc.lines[mid].start_ms<=ms)low=mid+1;else high=mid;}return low-1;}
export function changeLyricLine(line:LyricLine,text:string){return text===line.text?line:{...line,text,words:[]};}
export function formatStamp(ms:number){return `${Math.floor(ms/60000)}:${((ms%60000)/1000).toFixed(3).padStart(6,'0')}`;}
export function parseStamp(value:string){const m=/^(\d+):(\d{1,2}(?:\.\d{1,3})?)$/.exec(value);return m&&Number(m[2])<60?Math.round((Number(m[1])*60+Number(m[2]))*1000):null;}

export let requestedLyricEditor:number|undefined;
export function requestLyricEditor(id:number){requestedLyricEditor=id;window.dispatchEvent(new CustomEvent('musicos-edit-lyrics',{detail:id}));}

export function clearLyricEditorRequest(){requestedLyricEditor=undefined;}
