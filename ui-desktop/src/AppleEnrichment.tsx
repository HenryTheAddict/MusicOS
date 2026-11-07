import LyricEditionControl from './LyricEditionControl';
import{useEffect,useMemo,useRef,useState}from'react';
import{call,errorText,native,subscribe}from'./bridge';
import{Icon}from'./components';
import{useViewActive}from'./ViewScope';
import{requestAppleLyricReview,type LyricDocument}from'./lyrics';
import'./appleEnrichment.css';
export type AppleStatus={enabled:boolean;signed_in:boolean;sign_in:string;library:string;lyrics:string;artwork:string;phase:string;message:string;scanned:number;matched:number;ambiguous:number;completed:number;total:number;lyrics_ready:number;lyrics_failed:number;artwork_ready?:number;sidecars?:{written:number;unchanged:number;conflicts:number;failed:number}};
export type Transfer={id:string;name:string;kind:string;total:number;matched:number;missing:number;ambiguous:number;duplicates:number;available:boolean;ready:boolean;imported:boolean;missingAlbums?:{album:string;artist:string;needed:number;songs:string[]}[];reason?:string;checked?:boolean;resources?:{title:string;artist:string;kind:string}[]|null};
export type AppleResponse={sidecarTrackIds?:number[];playlistTotal?:number;transfers?:Transfer[];mostPlayed?:{trackId:number;title:string;artist:string;plays:number}[];status?:AppleStatus;choices?:RecordingChoice[];matches?:{trackId:number;catalogId:string;title:string;artist:string;album:string;cached:boolean}[];document?:LyricDocument;catalogId?:string;cached?:boolean;url?:string|null;staticPath?:string|null;state?:string;lowPower?:boolean};
type RecordingChoice = {
 trackId:number;catalogId:string;local?:{title:string;artist:string;album:string};
 item:{attributes?:{name?:string;artistName?:string;albumName?:string;durationInMillis?:number;contentRating?:string;discNumber?:number;trackNumber?:number}};
};
function RecordingChoices({choices,disabled,onSelect}:{choices:RecordingChoice[];disabled:boolean;onSelect:(choice:RecordingChoice)=>void}){
 const [query,setQuery]=useState('');
 const groups=useMemo(()=>{
  const bySong=new Map<number,Map<string,RecordingChoice>>();
  for(const choice of choices){
   const candidates=bySong.get(choice.trackId)??new Map<string,RecordingChoice>();
   candidates.set(choice.catalogId,choice);
   bySong.set(choice.trackId,candidates);
  }
  return [...bySong.values()].map(candidates=>[...candidates.values()]);
 },[choices]);
 if(!groups.length)return null;
 const needle=query.trim().toLocaleLowerCase();
 const visible=groups.filter(([choice])=>[choice.local?.title,choice.local?.artist,choice.local?.album,choice.item.attributes?.name].join(' ').toLocaleLowerCase().includes(needle));
 return <details className="apple-match-choices">
  <summary><Icon name="Back"/><span>Recording choices</span><small>{groups.length} {groups.length===1?'song':'songs'}</small></summary>
  <label className="apple-recording-search">Find a song <input type="search" placeholder="Title, artist or album" value={query} onChange={event=>setQuery(event.target.value)}/></label>
  <div className="apple-recording-groups">
   {visible.map(candidates=>{
    const first=candidates[0],local=first.local;
    return <details name="apple-local-recording" key={first.trackId}>
     <summary><Icon name="Back"/><span><strong>{local?.title??first.item.attributes?.name??'Song'}</strong><small>{[local?.artist,local?.album].filter(Boolean).join(' · ')}</small></span><small>{candidates.length} {candidates.length===1?'version':'versions'}</small></summary>
     <ul>{candidates.map(choice=>{
      const a=choice.item.attributes,rating=a?.contentRating==='explicit'?'Explicit':a?.contentRating==='clean'?'Clean':'Unrated';
      const seconds=a?.durationInMillis?Math.round(a.durationInMillis/1000):null;
      const duration=seconds===null?null:`${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`;
      return <li key={choice.catalogId}><div><strong>{rating}{duration&&` · ${duration}`}</strong>
        <small>{[a?.artistName,a?.albumName,a?.discNumber&&a.trackNumber?`Disc ${a.discNumber}, track ${a.trackNumber}`:null].filter(Boolean).join(' · ')}</small>
        <small>Apple Music ID {choice.catalogId}</small>
       </div><button disabled={disabled} onClick={()=>onSelect(choice)}>Use version</button></li>;
     })}</ul>
    </details>;
   })}
   {!visible.length&&<p>No recording choices match your search.</p>}
  </div>
 </details>;
}
let lastConnected=false,lastPhase='';
function publishStatus(status:AppleStatus,force=false){const changed=status.signed_in!==lastConnected||(status.phase==='connected'&&lastPhase!=='connected');lastConnected=status.signed_in;lastPhase=status.phase;if(changed||force)window.dispatchEvent(new CustomEvent('musicos-apple-status',{detail:status}));}
// Window-lifetime status delivery also reaches artwork while Settings is closed.
if(native)void subscribe<AppleStatus>('apple-enrichment-status',status=>publishStatus(status)).catch(()=>{});

export async function appleCall(action:string,trackId?:number,catalogId?:string){const result=await call('apple_enrichment',{request:{action,trackId,catalogId}});for(const id of result.sidecarTrackIds??[])window.dispatchEvent(new CustomEvent('musicos-lyrics-changed',{detail:id}));if(action==='select_match')window.dispatchEvent(new CustomEvent('musicos-lyrics-changed',{detail:trackId}));if(result.status)publishStatus(result.status,['sync','select_match','disconnect'].includes(action));return result;}
const fixture=new URLSearchParams(location.search).has('fixture')&&new URLSearchParams(location.search).has('appleFixture');
const initial:AppleStatus={enabled:false,signed_in:false,sign_in:'not_checked',library:'not_checked',lyrics:'not_checked',artwork:'not_checked',phase:'disconnected',message:'Optional, experimental, and entirely on your device.',scanned:0,matched:0,ambiguous:0,completed:0,total:0,lyrics_ready:0,lyrics_failed:0};
export default function AppleEnrichment(){
 const active=useViewActive(),[opened,setOpened]=useState(false),[status,setStatus]=useState(initial),[busy,setBusy]=useState(false),[error,setError]=useState(''),[choices,setChoices]=useState<AppleResponse['choices']>([]),[matches,setMatches]=useState<NonNullable<AppleResponse['matches']>>([]),[query,setQuery]=useState(''),generation=useRef(0);
 useEffect(()=>{if(!active||(!native&&!fixture))return;let alive=true;void appleCall('status').then(r=>{if(alive&&r.status)setStatus(r.status);}).catch(e=>{if(alive)setError(errorText(e));});return()=>{alive=false;};},[active]);
 useEffect(()=>{if(!active&&opened)setOpened(false);},[active,opened]);
 useEffect(()=>()=>{generation.current++;},[]);
 useEffect(()=>{if(!opened||!native)return;let disposed=false,off:(()=>void)|undefined;void subscribe<AppleStatus>('apple-enrichment-status',next=>{if(!disposed){setStatus(next);if(next.phase==='unavailable'||next.phase==='reconnect_required')setError(next.message);else if(next.signed_in&&next.phase!=='signing_in')setError('');}}).then(unlisten=>{if(disposed)unlisten();else off=unlisten;});return()=>{disposed=true;off?.();};},[opened]);

 useEffect(()=>{if(!opened||!active||(!native&&!fixture))return;let alive=true,timer=0;const deadline=performance.now()+600000;const poll=async()=>{try{const r=await appleCall('status');if(alive&&r.status)setStatus(r.status);}catch(e){if(alive){setError(errorText(e));return;}}if(alive&&(busy||['signing_in','background_cache'].includes(status.phase))&&(status.phase==='background_cache'||performance.now()<deadline))timer=window.setTimeout(poll,1000);};void poll();return()=>{alive=false;clearTimeout(timer);};},[opened,active,busy,status.phase]);
 useEffect(()=>{if(!opened||!active||(!native&&!fixture))return;let alive=true;void Promise.all([appleCall('choices'),appleCall('matches')]).then(([c,m])=>{if(alive){setChoices(c.choices??[]);setMatches(m.matches??[]);}}).catch(e=>{if(alive)setError(errorText(e));});return()=>{alive=false;};},[opened,active]);
 useEffect(()=>{if(!opened||status.phase!=='ready'||!status.lyrics_ready)return;let alive=true;void appleCall('matches').then(r=>{if(alive)setMatches(r.matches??[]);}).catch(()=>{});return()=>{alive=false;};},[opened,status.phase,status.lyrics_ready]);
 async function action(kind:string,trackId?:number,catalogId?:string){const seq=++generation.current;setBusy(true);setError('');if(['sync','sync_lyrics'].includes(kind))setStatus(s=>({...s,phase:kind==='sync'?'matching':'lyrics_sync',completed:0,total:kind==='sync_lyrics'?s.matched:0,message:kind==='sync'?'Reading your Apple library…':'Caching lyrics and syncing LyricsFiles…'}));try{if(!native&&!fixture)throw Error('Apple sign-in is available in the desktop app. This preview never accesses an account.');const r=await appleCall(kind,trackId,catalogId);if(seq!==generation.current)return;if(r.status)setStatus(r.status);if(['sync','sync_lyrics','select_match'].includes(kind)){const c=await appleCall('choices');if(seq===generation.current)setChoices(c.choices??[]);const matched=await appleCall('matches');if(seq===generation.current)setMatches(matched.matches??[]);}if(kind==='disconnect'){setChoices([]);setMatches([]);}}catch(e){if(seq===generation.current){setError(errorText(e));void appleCall('status').then(r=>{if(seq===generation.current&&r.status)setStatus(r.status);}).catch(()=>{});}}finally{if(seq===generation.current)setBusy(false);}}
 async function review(trackId:number){setBusy(true);setError('');try{const r=await appleCall('lyrics',trackId);if(r.document){setOpened(false);requestAppleLyricReview(trackId,r.document);}}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 const working=['matching','lyrics_sync','collections_sync','background_cache'].includes(status.phase),step=status.signed_in?status.library==='available'?2:1:0;
 return <article className="apple-integration" aria-label="Apple Music integration"><header className="apple-integration-header"><Icon name="Lyrics"/><div><h3>Apple Music</h3><p>{status.signed_in?'Connected · lyrics and artwork for matching songs':'Optional lyrics, artwork, and playlist matching'}</p></div><button aria-expanded={opened} onClick={e=>{e.currentTarget.focus({preventScroll:true});setOpened(v=>!v);}}>{opened?'Hide details':'Manage Apple Music'}</button></header>
 {opened&&<section className="apple-integration-details" aria-label="Apple Music connection"><h4>Connection</h4><p>{step===0?'Sign in in Apple’s private window.':'Match recordings to songs already in your musicOS library.'}</p>
 <div className="apple-capabilities">{([['sign_in','Sign-in'],['library','Library'],['lyrics','Word lyrics'],['artwork','Animated art']] as const).map(([key,label])=><div key={key} data-state={status[key]}><span className="apple-capability-dot"/><span>{label}</span><strong>{({available:'Ready',stored:'Saved session',unavailable:'Unavailable',waiting:'Waiting',not_checked:'Not checked'} as Record<string,string>)[status[key]]??'Not checked'}</strong></div>)}</div>
 <div className="apple-connection-actions">{(step===0||status.phase==='unavailable'||status.phase==='reconnect_required'||status.phase==='signing_in')&&<><button className="apple-primary" disabled={busy} onClick={()=>void action('begin')}><Icon name="Library"/>{status.phase==='signing_in'?'Return to Apple sign-in':status.signed_in?'Reconnect':'Open Apple sign-in'}</button>{status.phase==='signing_in'&&<button disabled={busy} onClick={()=>void action('verify')}>Check connection</button>}</>}</div>
 <h4 className="apple-group-heading">Lyrics &amp; artwork</h4><LyricEditionControl/>
 <div className="apple-setup-actions">{step!==0&&<><button className="apple-primary" disabled={busy||working} onClick={()=>void action(step===1?'sync':'start_cache')}><Icon name="sync"/>{step===1?'Find matches':'Save lyrics & artwork'}</button>{step===2&&<button disabled={busy||working} onClick={()=>void action('sync')}>Refresh matches</button>}</>}{(working||status.phase==='signing_in')&&<button onClick={()=>void action(status.phase==='background_cache'?'stop_cache':'cancel')}>Cancel</button>}</div>
 <div className="apple-sync-status" role="status">
  {status.signed_in&&<dl>
   <div><dt>Matched songs</dt><dd>{status.matched}</dd></div>
   {(status.artwork_ready!==undefined||status.lyrics_ready>0)&&<>
    <div><dt>Lyrics cached</dt><dd>{status.lyrics_ready}</dd></div>
    <div><dt>Artwork cached</dt><dd>{status.artwork_ready??0}</dd></div>
    {status.lyrics_failed>0&&<div><dt>Lyrics unavailable</dt><dd>{status.lyrics_failed}</dd></div>}
   </>}
  </dl>}
  <p>{working?(status.phase==='matching'?`Finding matches · ${status.scanned} Apple Music songs checked`:status.total?`${status.completed} of ${status.total} songs checked`:'Preparing…'):status.message}</p>
  {status.sidecars&&<p>LyricsFiles · {status.sidecars.written} saved · {status.sidecars.unchanged} current
    {status.sidecars.conflicts>0&&` · ${status.sidecars.conflicts} edits kept`}{status.sidecars.failed>0&&` · ${status.sidecars.failed} errors`}</p>}
  {working&&<progress aria-label="Apple lyric sync progress" max={status.total||undefined} value={status.phase!=='matching'&&status.total?status.completed:undefined}/>}
 </div>
 {error&&<div className="apple-error" role="alert"><strong>Couldn’t complete this action</strong><p>{error}</p><p>Local files and LRCLIB remain available.</p></div>}
 <details className="apple-detail-group"><summary><Icon name="Back"/><span>Playlists & listening</span></summary><AppleTransfers connected={status.signed_in} locked={working||busy}/></details>
 <RecordingChoices choices={choices??[]} disabled={busy||working} onSelect={choice=>void action('select_match',choice.trackId,choice.catalogId)}/>
 {!!matches.length&&<details className="apple-review-list"><summary><Icon name="Back"/><span>Matched songs · {matches.length}</span></summary><label>Find a match<input type="search" value={query} onChange={e=>setQuery(e.target.value)} placeholder="Song, artist or album"/></label><div>{matches.filter(m=>`${m.title} ${m.artist} ${m.album}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())).map(m=><button key={m.trackId} disabled={busy||working} onClick={()=>void review(m.trackId)}><span><strong>{m.title}</strong><small>{m.artist} · {m.album}</small></span><span>{m.cached?'Review lyrics':'Find lyrics'}</span></button>)}</div></details>}
 <footer className="apple-setup-footer"><p>Experimental · capabilities are checked independently.</p>{status.enabled&&<button disabled={working} onClick={()=>void action('disconnect')}>Disconnect</button>}</footer></section>}</article>;

}

function AppleTransfers({connected,locked}:{connected:boolean;locked:boolean}){
 const [data,setData]=useState<AppleResponse>({}),[busy,setBusy]=useState(false),[error,setError]=useState(''),[message,setMessage]=useState('');
 useEffect(()=>{let alive=true;void appleCall('transfers').then(r=>{if(alive)setData(r);}).catch(e=>{if(alive)setError(errorText(e));});return()=>{alive=false;};},[]);
 async function run(action:string,id?:string){setBusy(true);setError('');setMessage('');try{setData(await appleCall(action,undefined,id));setMessage(action==='transfer_playlist'?'Local playlist created. Your existing collections were kept.':'Collections cached for offline review.');}catch(e){setError(errorText(e));}finally{setBusy(false);}}
 return <section className="apple-transfers" aria-label="Playlists and listening"><header><h3>Playlists & listening</h3><button disabled={busy||locked||!connected} onClick={()=>void run('refresh_transfers')}><Icon name="sync"/>{busy?'Working…':'Refresh collections'}</button>{busy&&<button onClick={()=>void appleCall('cancel')}>Cancel</button>}</header><p>Eight recently updated playlists, with songs loaded only when you check one. Transfer only when every entry has a unique local match.</p>{!!data.playlistTotal&&<p>Showing up to 8 of {data.playlistTotal} playlists.</p>}{error&&<p role="alert">{error}</p>}{message&&<p role="status">{message}</p>}{data.transfers?.map(t=><article key={t.id}><div><strong>{t.name}</strong><small>{t.checked===false?'Song matches not checked':`${t.matched} of ${t.total} locally matched`}{t.missing?` · ${t.missing} missing`:''}{t.ambiguous?` · ${t.ambiguous} ambiguous`:''}{t.duplicates?` · ${t.duplicates} repeated entries`:''}</small>{t.reason&&<small>{t.reason}</small>}{!!t.missingAlbums?.length&&<details className="needed-albums"><summary><Icon name="Back"/><span>Albums needed · {t.missingAlbums.length}</span></summary><p>These playlist entries need a local copy or a recording choice. Nothing is downloaded automatically.</p>{t.missingAlbums.map(a=><div key={`${a.artist}:${a.album}`}><strong>{a.album}</strong><small>{a.artist} · {a.needed} {a.needed===1?'song':'songs'} needed</small><small>{a.songs.join(' · ')}{a.needed>a.songs.length?' · …':''}</small></div>)}</details>}{!!t.resources?.length&&<details><summary><Icon name="Back"/><span>View listening summary</span></summary><ol>{t.resources.map((r,i)=><li key={i}><span><strong>{r.title}</strong><small>{r.artist} · {r.kind.replace('library-','')}</small></span></li>)}</ol></details>}</div><button disabled={busy||locked||t.imported||(t.checked===false?!connected:!t.ready)} onClick={()=>void run(t.checked===false?'check_playlist':'transfer_playlist',t.id)}>{t.imported?'Transferred':t.checked===false?'Check songs':t.ready?'Transfer playlist':!t.available?'Summary only':'Needs matches'}</button></article>)}
 {!!data.mostPlayed?.length&&<details><summary><Icon name="Back"/><span>Most played in musicOS</span></summary><ol>{data.mostPlayed.map(t=><li key={t.trackId}><span>{t.title}<small>{t.artist}</small></span><strong>{t.plays} plays</strong></li>)}</ol><p>Actual musicOS play counts. Apple listening summaries do not change these.</p></details>}</section>;
}
