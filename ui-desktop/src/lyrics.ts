import {useEffect,useRef,useState} from 'react';
import {useViewActive} from './ViewScope';
import {call,errorText} from './bridge';
export type LyricWord={ [key:string]:unknown;text:string;start_ms:number;end_ms?:number};
export type LyricLine={ [key:string]:unknown;'x-musicos'?:{voice?:'lead'|'second'|'both'|'unassigned';agents?:string[]};text:string;start_ms:number;end_ms?:number;words?:LyricWord[]};
export type LyricDocument={[key:string]:unknown;'x-musicos'?:{[key:string]:unknown;attribution?:{provider:string;catalogId?:string};singers?:Record<string,{name:string;group?:boolean}>;original?:{format:string;text:string}};version:'1.0';metadata:{title:string;artist:string;album:string;duration_ms?:number;offset_ms:number;language?:string;instrumental:boolean};lines:LyricLine[];plain:string};
export type LyricCandidate={id:number;title:string;artist:string;album:string;duration:number;document:LyricDocument};
export type LyricResponse={before?:LyricDocument|null;after?:LyricDocument|null;track_id:number;revision:number;document:LyricDocument|null;source:{kind:string;label:string;url:string|null;provider_id:number|null}|null;state:string;message?:string;retry_after_seconds?:number;provider_status?:number;candidates:LyricCandidate[];history?:{id:number;track_id:number;at:number;revision:number;undone:boolean}[]};
export type LyricRequest={action:string;trackId?:number;revision?:number;document?:LyricDocument;candidateId?:number;text?:string;format?:'lyricsfile'|'lrc'|'plain'|'ttml';path?:string;overwrite?:boolean;historyId?:number;retry?:boolean;writeFiles?:boolean};
const pending=new Map<string,Promise<LyricResponse>>();
export function lyricCall(request:LyricRequest,identity=''){const key=JSON.stringify({...request,_identity:identity});if(['load','lookup','search','find'].includes(request.action)){const old=pending.get(key);if(old)return old;const job=call('lyrics',{request}).finally(()=>{if(pending.get(key)===job)pending.delete(key);});pending.set(key,job);return job;}return call('lyrics',{request});}
type Session={trackId:number;result:LyricResponse|null;error:string;at:number;retryAt:number};
const sessionPolicy={maxRecords:64,readyMs:5*60_000,missMs:24*60*60_000,failureMs:30_000};
const sessions=new Map<string,Session>(),loads=new Map<string,Promise<Session>>(),versions=new Map<number,number>();
export function clearLyricSession(trackId?:number){if(trackId===undefined){sessions.clear();loads.clear();pending.clear();versions.clear();return;}versions.set(trackId,(versions.get(trackId)??0)+1);for(const [key,value] of sessions)if(value.trackId===trackId)sessions.delete(key);for(const key of loads.keys())if(key.startsWith(`${trackId}:`))loads.delete(key);for(const key of pending.keys())if(JSON.parse(key).trackId===trackId)pending.delete(key);}
if(typeof window!=='undefined')window.addEventListener('musicos-lyrics-changed',event=>clearLyricSession((event as CustomEvent<number>).detail));
const shown=(entry:Session)=>entry.result?.state==='retry_later'?{...entry.result,retry_after_seconds:Math.max(0,Math.ceil((entry.retryAt-Date.now())/1000))}:entry.result;
function remember(key:string,trackId:number,result:LyricResponse|null,error=''){
 const at=Date.now(),entry={trackId,result,error,at,retryAt:at+(result?.retry_after_seconds??0)*1000};sessions.delete(key);sessions.set(key,entry);while(sessions.size>sessionPolicy.maxRecords)sessions.delete(sessions.keys().next().value!);
 return entry;
}
function loadSession(key:string,trackId:number,online:boolean){const old=loads.get(key);if(old)return old;const version=versions.get(trackId)??0;const job=(async()=>{let r=await lyricCall({action:'load',trackId},key);if(!r.document&&online&&r.state==='missing'){r=await lyricCall({action:'lookup',trackId},key);}return (versions.get(trackId)??0)===version?remember(key,trackId,r):{trackId,result:r,error:'',at:Date.now(),retryAt:0};})().catch(e=>{if((versions.get(trackId)??0)===version)return remember(key,trackId,null,errorText(e));throw e;}).finally(()=>{if(loads.get(key)===job)loads.delete(key);});loads.set(key,job);return job;}
export function useLyrics(trackId:number|undefined,active:boolean,online=true,identity=''){
 const visible=useViewActive();active=active&&visible;const [result,setResult]=useState<LyricResponse|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),generation=useRef(0),[epoch,setEpoch]=useState(0),[retryIn,setRetryIn]=useState(0),retryDeadline=useRef(0),intent=useRef(0),lastKey=useRef('');const key=`${trackId}:${online}:${identity}`;
 useEffect(()=>{if(!active)return;const changed=(event:Event)=>{if((event as CustomEvent<number>).detail===trackId)setEpoch(v=>v+1);};window.addEventListener('musicos-lyrics-changed',changed);return()=>window.removeEventListener('musicos-lyrics-changed',changed);},[active,trackId]);
 useEffect(()=>{const revision=++generation.current;intent.current=0;const sameIdentity=lastKey.current===key;lastKey.current=key;setError('');setResult(previous=>sameIdentity&&previous?.track_id===trackId?previous:null);if(!active||!trackId){setBusy(false);return()=>{generation.current++;};}const apply=(entry:Session)=>{if(revision!==generation.current)return;retryDeadline.current=entry.retryAt;setResult(shown(entry));setError(entry.error);setBusy(false);};const cached=sessions.get(key);if(cached){apply(cached);const ttl=cached.result?.document?sessionPolicy.readyMs:cached.result?.state==='not_found'?sessionPolicy.missMs:sessionPolicy.failureMs;if(Date.now()-cached.at<ttl)return()=>{generation.current++;};}setBusy(!(result?.track_id===trackId&&!!result?.document));void loadSession(key,trackId,online).then(apply).catch(e=>{if(revision===generation.current){setError(errorText(e));setBusy(false);}});return()=>{generation.current++;};},[key,trackId,active,online,epoch]);
 // A countdown enables explicit Retry; it never starts provider work itself.
 useEffect(()=>{if(!active||!result||result.track_id!==trackId||result.state!=='retry_later'){setRetryIn(0);return;}let timer=0;const tick=()=>{const left=Math.max(0,Math.ceil((retryDeadline.current-Date.now())/1000));setRetryIn(left);if(left)timer=window.setTimeout(tick,Math.min(1000,Math.max(1,retryDeadline.current-Date.now())));};tick();return()=>clearTimeout(timer);},[result,active,trackId]);
 async function find(search=false){if(!active||!trackId||retryIn>0||intent.current)return;const revision=++generation.current;intent.current=revision;setBusy(true);setError('');try{const r=await lyricCall({action:search?'search':'lookup',trackId,retry:true},key);if(revision===generation.current){const entry=remember(key,trackId,r);retryDeadline.current=entry.retryAt;setResult(shown(entry));}}catch(e){if(revision===generation.current)setError(errorText(e));}finally{if(intent.current===revision)intent.current=0;if(revision===generation.current)setBusy(false);}}
 async function select(id:number){if(!result||!trackId)return;const revision=++generation.current;setBusy(true);try{const selected=await lyricCall({action:'select',trackId,revision:result.revision,candidateId:id});if(revision!==generation.current)return;setResult(selected);window.dispatchEvent(new CustomEvent('musicos-lyrics-changed',{detail:trackId}));}catch(e){if(revision===generation.current)setError(errorText(e));}finally{if(revision===generation.current)setBusy(false);}}
 return {result,busy,error,find,select,retryIn};
}
export function lyricIndex(doc:LyricDocument,position:number){const ms=position*1000-doc.metadata.offset_ms;let low=0,high=doc.lines.length;while(low<high){const mid=(low+high)>>>1;if(doc.lines[mid].start_ms<=ms)low=mid+1;else high=mid;}while(low>0&&!visibleLyricLine(doc,low-1))low--;return low-1;}
export function changeLyricLine(line:LyricLine,text:string){return text===line.text?line:{...line,text,words:[]};}
export function lyricText(doc:LyricDocument){return doc.lines.length?doc.lines.map(line=>line.text).join('\n'):doc.plain;}
/** Keep exact unchanged prefixes/suffixes and their imported word timestamps.
 * Inserted lines share the preceding time until the author stamps them. */
export function editLyricText(doc:LyricDocument,text:string):LyricDocument{
 if(text===lyricText(doc))return doc;
 if(!doc.lines.length)return {...doc,plain:text};
 const words=text.split('\n'),old=doc.lines;let head=0,tail=0;
 while(head<Math.min(words.length,old.length)&&words[head]===old[head].text)head++;
 while(tail<Math.min(words.length-head,old.length-head)&&words[words.length-1-tail]===old[old.length-1-tail].text)tail++;
 const lines=old.slice(0,head),end=old.length-tail;
 for(let i=head;i<words.length-tail;i++)lines.push(i<end?changeLyricLine(old[i],words[i]):{text:words[i],start_ms:lines.at(-1)?.start_ms??0});
 lines.push(...old.slice(end));
 return {...doc,plain:text,lines};
}
/** Empty duplicate LRC cues are formatting, not an instrumental break. */
export function visibleLyricLine(doc:LyricDocument,index:number){const line=doc.lines[index];return !!line.text.trim()||!(doc.lines[index+1]?.start_ms===line.start_ms||doc.lines[index-1]?.start_ms===line.start_ms);}
export function formatStamp(ms:number){return `${Math.floor(ms/60000)}:${((ms%60000)/1000).toFixed(3).padStart(6,'0')}`;}
export function parseStamp(value:string){const m=/^(\d+):(\d{1,2}(?:\.\d{1,3})?)$/.exec(value);return m&&Number(m[2])<60?Math.round((Number(m[1])*60+Number(m[2]))*1000):null;}

export let requestedLyricEditor:number|undefined;
export function requestLyricEditor(id:number){requestedLyricEditor=id;window.dispatchEvent(new CustomEvent('musicos-edit-lyrics',{detail:id}));}

export function clearLyricEditorRequest(){requestedLyricEditor=undefined;}

/** A later cue does not end a voice whose explicit interval is still active. */
const cueEnds=new WeakMap<LyricDocument,number[]>();
export function activeLyricIndices(doc:LyricDocument,position:number){let ends=cueEnds.get(doc);if(!ends){ends=[];let next=doc.metadata.duration_ms??Infinity,group=Infinity;for(let i=doc.lines.length-1;i>=0;i--){const line=doc.lines[i];if(line.start_ms<group){if(group!==Infinity)next=group;group=line.start_ms;}ends[i]=line.end_ms??next;}cueEnds.set(doc,ends);}const ms=position*1000-doc.metadata.offset_ms;return doc.lines.flatMap((line,i)=>visibleLyricLine(doc,i)&&ms>=line.start_ms&&ms<ends![i]?[i]:[]);}

export function lyricVoice(line:LyricLine){return line['x-musicos']?.voice??(line['x-musicos']?.agents?.length?'unassigned':'lead');}
export function lossyLrc(doc:LyricDocument){return doc.lines.some((l,i)=>l.words?.length||l['x-musicos']||l.end_ms!==undefined&&doc.lines.slice(i+1).some(n=>n.start_ms<l.end_ms!));}

let pendingAppleDraft:{trackId:number;document:LyricDocument}|undefined;
export function requestAppleLyricReview(trackId:number,document:LyricDocument){pendingAppleDraft={trackId,document};window.dispatchEvent(new CustomEvent('musicos-open-lyric-editor',{detail:trackId}));}
export function takeAppleLyricDraft(trackId:number,consume?:LyricDocument){if(pendingAppleDraft?.trackId!==trackId)return;const doc=pendingAppleDraft.document;if(consume===doc)pendingAppleDraft=undefined;return doc;}

export function lyricSpeaker(doc:LyricDocument,line:LyricLine){const names=line['x-musicos']?.agents?.map(id=>doc['x-musicos']?.singers?.[id]?.name??id).filter(Boolean);return names?.length?names.join(' & '):lyricVoice(line)==='second'?'Second voice':lyricVoice(line)==='both'?'Both voices':'Lead';}
