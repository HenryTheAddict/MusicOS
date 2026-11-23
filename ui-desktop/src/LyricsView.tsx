import {useLyricGlow} from './useLyricGlow';
import {lyricWordSegments,lyricWordEnd,lyricWordActive,wordProgress} from './lyricWords';
import {useLyricFollow} from './useLyricFollow';
import {appleCall} from './AppleEnrichment';
import './lyrics.css';
import {Fragment,useEffect,useLayoutEffect,useMemo,useRef,useState,type ReactNode} from 'react';
import {resolveArtworkTheme} from './artworkTheme';
import LyricActivity from './LyricActivity';
import {Icon,Overflow,type MenuItem} from './components';
import {call,errorText} from './bridge';
import {clockPosition,currentPlaybackClock,subscribePlaybackClock} from './playbackFeed';
import {useViewActive} from './ViewScope';
import {activeLyricIndices,lyricSpeaker,lyricVoice,lyricIndex,lyricPauses,lyricPauseAt,visibleLyricLine,useLyrics,lyricCall,type LyricDocument} from './lyrics';
import type {Playback,Track} from './types';
export function LiquidScene({active,moving,playing,cover}:{active:boolean;moving:boolean;playing:boolean;cover?:string|null}){
 const host=useRef<HTMLDivElement>(null),[artColors,setArtColors]=useState<string[]|null>(null),state=useRef({phase:0,energy:[0,0,0]}),sceneColors=useRef(['#0064df','#54b9ff','#0064df']),painted=useRef(false);
 useEffect(()=>{let alive=true;if(active&&!cover)setArtColors(null);if(active&&cover)void resolveArtworkTheme([cover]).then(value=>{if(alive)setArtColors(value?(value.palette.swatches?.length===3?value.palette.swatches:[value.palette.accent,value.palette.blob,value.palette.background]):null);});return()=>{alive=false;};},[active,cover]);
 useEffect(()=>{const node=host.current,canvas=node?.querySelector('canvas'),ctx=canvas?.getContext('2d');if(!node||!canvas||!ctx||!active)return;ctx.setTransform(canvas.width/640,0,0,canvas.height/480,0,0);node.dataset.renderer='canvas';node.dataset.soft='filter' in ctx?'canvas':'compositor';if('filter' in ctx)ctx.filter='blur(12px)';const media=matchMedia('(prefers-reduced-motion: reduce)');let frame=0,settleTimer=0,last=0,nextPaint=0,settleUntil=performance.now()+180,colorStarted=0,from=[...sceneColors.current],colors=[...sceneColors.current];
  const palette=()=>{const style=getComputedStyle(node);from=[...sceneColors.current];colorStarted=performance.now();colors=(artColors??['--accent','--blob-color','--playback-tone'].map((key,i)=>style.getPropertyValue(key).trim()||colors[i])).map((color,i)=>{
    if(/^#[\da-f]{6}$/i.test(color))return color;
    ctx.fillStyle='#0064df';ctx.fillStyle=color;const normalized=String(ctx.fillStyle);
    if(/^#[\da-f]{6}$/i.test(normalized))return normalized;
    const rgb=normalized.match(/^rgba?\(([^)]+)\)/),srgb=normalized.match(/^color\(srgb\s+([^)]+)\)/);
    const channels=(rgb?.[1]??srgb?.[1])?.match(/[-+]?(?:\d*\.)?\d+/g)?.slice(0,3).map(Number);
    return channels?.length===3?'#'+channels.map(v=>Math.round(Math.max(0,Math.min(255,v*(srgb?255:1)))).toString(16).padStart(2,'0')).join(''):sceneColors.current[i];
   });if(!painted.current||media.matches){from=[...colors];sceneColors.current=[...colors];painted.current=true;}if(from.every((color,i)=>color===colors[i]))colorStarted=-Infinity;};
  const paint=()=>{
   const elapsed=media.matches?1:Math.min(1,(performance.now()-colorStarted)/420),mix=elapsed*elapsed*(3-2*elapsed);
   sceneColors.current=elapsed===1?colors:colors.map((color,i)=>'#'+[1,3,5].map(j=>Math.round(parseInt(from[i].slice(j,j+2),16)*(1-mix)+parseInt(color.slice(j,j+2),16)*mix).toString(16).padStart(2,'0')).join(''));
   const {phase,energy}=state.current;ctx.clearRect(0,0,640,480);
   for(let i=0;i<3;i++){
    const a=phase+i*2.1,bass=energy[0],detail=energy[i],wave=Math.sin(a*1.4),bend=Math.cos(a*.9);
    ctx.save();ctx.translate([105,510,320][i]+Math.sin(a)*100*(.45+bass*1.3),[130,280,455][i]+Math.cos(a*.7)*90*(.45+bass*1.3));ctx.rotate(Math.sin(a*.8)*(.18+bass*.18));ctx.scale(1+detail*.42,1+Math.sin(a)*.08);
    const blob=new Path2D(`M-220 -25 C${-185+wave*45} ${-205-detail*75},${80+bend*60} ${-200+bass*65},220 -45 C${300+detail*75} ${60+wave*35},110 ${180+bass*75},-35 175 C${-210-bass*60} ${205+bend*40},-300 ${55-detail*70},-220 -25Z`);
    const gradient=ctx.createRadialGradient(-20,-20,10,0,0,310);gradient.addColorStop(0,sceneColors.current[i]);gradient.addColorStop(.25,sceneColors.current[i]);gradient.addColorStop(1,'transparent');ctx.globalAlpha=[.65,.58,.54][i];ctx.fillStyle=gradient;ctx.fill(blob);ctx.restore();
   }
   ctx.globalAlpha=1;
  };
  const draw=(at:number)=>{frame=0;if(document.hidden)return;const clock=currentPlaybackClock(),live=playing&&clock.playing&&moving&&!media.matches;if(at<nextPaint){frame=requestAnimationFrame(draw);return;}nextPaint=at+30;const dt=Math.min(75,at-(last||at));last=at;for(let i=0;i<3;i++){const target=Math.max(0,Math.min(1,live?(clock.levels[i]??0):0)),previous=state.current.energy[i];state.current.energy[i]+=(target-previous)*(1-Math.exp(-dt/(target>previous?90:220)));}if(moving&&!media.matches)state.current.phase+=dt*.00024*(.4+state.current.energy[0]*2);paint();if(!media.matches&&(at<colorStarted+320||live||(moving&&state.current.energy.some(e=>e>.001)&&at<settleUntil)))frame=requestAnimationFrame(draw);};
  const start=()=>{cancelAnimationFrame(frame);clearTimeout(settleTimer);last=0;nextPaint=0;settleUntil=performance.now()+180;palette();if(!document.hidden){paint();if(!media.matches)frame=requestAnimationFrame(draw);if(!playing)settleTimer=window.setTimeout(()=>{state.current.energy.fill(0);if(!document.hidden)paint();},180);}};
  const off=subscribePlaybackClock(()=>{if(!frame&&playing&&moving&&!media.matches&&!document.hidden&&currentPlaybackClock().playing)start();});
  const observer=new MutationObserver(start);const dialog=node.closest('dialog');if(dialog)observer.observe(dialog,{attributes:true,attributeFilter:['style']});document.addEventListener('visibilitychange',start);media.addEventListener('change',start);start();return()=>{off();cancelAnimationFrame(frame);clearTimeout(settleTimer);observer.disconnect();document.removeEventListener('visibilitychange',start);media.removeEventListener('change',start);};
 },[active,moving,playing,artColors]);
 return <div ref={host} className="lyrics-liquid" aria-hidden="true"><><canvas width={384} height={288}/></><svg viewBox="0 0 640 480" preserveAspectRatio="xMidYMid slice"><defs><filter id="lyric-soft"><feGaussianBlur stdDeviation="30"/></filter></defs><g filter="url(#lyric-soft)"><ellipse cx="110" cy="160" rx="170" ry="120"/><ellipse cx="490" cy="290" rx="180" ry="130"/><ellipse cx="320" cy="450" rx="180" ry="150"/></g></svg></div>;
}
export default function LyricsView({track,player,active,onSeek,onPlay,onEdit,variant='full',renderMenu}:{track:Track|undefined;player:Playback;active:boolean;onSeek:(seconds:number)=>Promise<unknown>;onPlay:()=>Promise<void>;onEdit?:()=>void;variant?:'full'|'panel'|'mini'|'visualizer';renderMenu?:(items:MenuItem[])=>ReactNode}){const visible=useViewActive();active=active&&visible;const {result,busy,error,find,select,retryIn}=useLyrics(track?.id,active,true,track?JSON.stringify([track.path,track.title,track.artist,track.album,track.duration]):''),scroll=useRef<HTMLDivElement>(null),[following,setFollowing]=useState(true),[position,setPosition]=useState(0),[playError,setPlayError]=useState(''),[starting,setStarting]=useState(false);const pane=useRef<HTMLElement>(null),longPress=useRef(0),touchOrigin=useRef({x:0,y:0});useEffect(()=>()=>clearTimeout(longPress.current),[active]);
 const edit=()=>{if(onEdit)onEdit();else if(track)window.dispatchEvent(new CustomEvent('musicos-open-lyric-editor',{detail:track.id}));};
 const context=(point?:{x:number;y:number})=>pane.current?.querySelector<HTMLButtonElement>('.context-only,button[aria-haspopup=menu]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:point}));
 const [appleCandidate,setAppleCandidate]=useState<{trackId:number;doc:LyricDocument}|null>(null),[appleBusy,setAppleBusy]=useState(false),[appleError,setAppleError]=useState(''),appleIntent=useRef(0);useEffect(()=>{appleIntent.current++;setAppleCandidate(null);setAppleError('');setAppleBusy(false);},[track?.id,track?.title,track?.artist,track?.album,track?.duration,active]);
 async function findApple(){if(!track||appleBusy)return;const token=++appleIntent.current;setAppleBusy(true);setAppleError('');try{const r=await appleCall('lyrics',track.id);if(token===appleIntent.current&&r.document)setAppleCandidate({trackId:track.id,doc:r.document});}catch(e){if(token===appleIntent.current)setAppleError(errorText(e));}finally{if(token===appleIntent.current)setAppleBusy(false);}}
 async function selectApple(){if(appleBusy||!appleCandidate||!track||appleCandidate.trackId!==track.id)return;const token=++appleIntent.current,id=track.id;setAppleBusy(true);try{await lyricCall({action:'save',trackId:id,revision:result?.revision??0,document:appleCandidate.doc});if(token===appleIntent.current)setAppleCandidate(null);window.dispatchEvent(new CustomEvent('musicos-lyrics-changed',{detail:id}));}catch(e){if(token===appleIntent.current)setAppleError(errorText(e));}finally{if(token===appleIntent.current)setAppleBusy(false);}}
 const current=track?.id===player.track?.id,doc=result?.track_id===track?.id?result?.document:null;
 useEffect(()=>{setFollowing(true);},[track?.id]);
 const pauses=useMemo(()=>doc?lyricPauses(doc,track?.duration??0):[],[doc,track?.duration]);
 const pausesByLine=useMemo(()=>new Map(pauses.map(item=>[item.before,item])),[pauses]);
 const pause=current?lyricPauseAt(pauses,position):undefined;
 const wordSegments=useMemo(()=>doc?.lines.map(lyricWordSegments)??[],[doc]);
 const wordSignature=doc&&!pause?activeLyricIndices(doc,position).join('|'):'';
 const glowTick=useRef<(time:number)=>void>(()=>{}),glowPosition=useRef<()=>void>(()=>{});
 const updateClock=useRef<(()=>void)|null>(null);
 const displayedKey=useRef(''),displayedDoc=useRef(doc);if(displayedDoc.current!==doc){displayedDoc.current=doc;displayedKey.current='';}
 useLayoutEffect(()=>{
  if(!active||!current||!doc?.lines.length)return;
  let frame=0,off:undefined|(()=>void),lastPane:string|undefined;
  let words:{node:HTMLElement;start:number;end:number|undefined;progress:number;active:boolean;sung:boolean}[]=[];
  const update=()=>{
   if(document.hidden)return;
   const value=currentPlaybackClock();
   if(value.trackId!==track?.id)return;
   const pos=clockPosition(value),lyricTime=pos*1000-doc.metadata.offset_ms;
   // React commits active cues between frames. Re-read their elements only
   // when the actual set of rendered lines changes.
   const shown=pane.current?.dataset.currentCues;
   if(shown!==lastPane){
    lastPane=shown;
    words=Array.from(pane.current?.querySelectorAll<HTMLElement>('.lyric-line[data-current=true] .lyric-word')??[],node=>({
     node,start:Number(node.dataset.wordStart),end:node.dataset.wordEnd===undefined?undefined:Number(node.dataset.wordEnd),
     progress:-1,active:node.dataset.wordActive==='true',sung:node.dataset.sung==='true',
    }));
   }
   for(const word of words){
    const progress=Math.round(wordProgress(word.start,word.end,lyricTime)*1000)/10;
    if(progress!==word.progress){
     const feather=progress>0&&progress<100,previous=word.progress>0&&word.progress<100;
     word.node.style.setProperty('--word-progress',`${progress}%`);
     if(variant==='full'){
      const sung=progress/100;
      const eased=sung*sung*(3-2*sung);
      word.node.style.setProperty('--word-weight',String(Math.round(300+eased*400)));
      word.node.style.setProperty('--word-ink',String(.82+eased*.18));
     }
     if(word.progress<0||previous!==feather)word.node.style.setProperty('--word-feather',feather?'8%':'0%');
     word.progress=progress;
    }
    const sung=lyricTime>=word.start,active=sung&&(word.end===undefined||lyricTime<word.end);
    if(sung!==word.sung){if(sung)word.node.dataset.sung='true';else delete word.node.dataset.sung;word.sung=sung;}
    if(active!==word.active){if(active)word.node.dataset.wordActive='true';else delete word.node.dataset.wordActive;word.active=active;}
   }
   if(words.length)glowTick.current(lyricTime);
   const key=`${track?.id}:${lyricIndex(doc,pos)}:${activeLyricIndices(doc,pos).join(',')}:${lyricPauseAt(pauses,pos)?.before??''}`;
   if(key!==displayedKey.current){displayedKey.current=key;setPosition(pos);}
   if(value.playing&&words.length&&!frame)frame=requestAnimationFrame(()=>{frame=0;update();});
  };
  const visibility=()=>{off?.();off=undefined;cancelAnimationFrame(frame);frame=0;if(!document.hidden){off=subscribePlaybackClock(update);update();}};
  updateClock.current=update;
  visibility();
  document.addEventListener('visibilitychange',visibility);
  return()=>{updateClock.current=null;off?.();cancelAnimationFrame(frame);document.removeEventListener('visibilitychange',visibility);};
 },[active,current,player.is_playing,track?.id,doc,pauses,variant]);

 const index=doc&&current?lyricIndex(doc,position):-1,activeLines=doc&&current&&!pause?activeLyricIndices(doc,position):[],activeKey=activeLines.join(','),activeSet=new Set(activeLines),focusLines=activeLines.length?activeLines:[Math.max(0,index)];
 const nextCue=doc?.lines.findIndex((_,i)=>i>index&&visibleLyricLine(doc,i));
 const lineStyle=(i:number)=>{
  const distance=Math.min(...focusLines.map(n=>Math.abs(i-n)));
  if(variant==='full'){
   const cueDistance=pause?Math.abs(i-pause.before+.5):distance;
   return {'--lyric-distance':cueDistance} as import('react').CSSProperties;
  }
  return {'--lyric-distance':Math.min(...focusLines.map(n=>Math.max(0,i-n))),'--lyric-blur':`${Math.min(3.2,distance*.65)}px`} as import('react').CSSProperties;
 };
 const [layoutEpoch,setLayoutEpoch]=useState(0);
 useLayoutEffect(()=>{const host=scroll.current;if(!active||!host)return;let size='';const measure=()=>{const next=host.clientHeight,key=`${next}:${host.clientWidth}`;if(key===size)return;size=key;host.style.setProperty('--lyric-space',`${next*.5}px`);setLayoutEpoch(v=>v+1);};measure();const observer=new ResizeObserver(measure);observer.observe(host);const fonts=()=>{size='';measure();};document.fonts?.addEventListener('loadingdone',fonts);return()=>{observer.disconnect();document.fonts?.removeEventListener('loadingdone',fonts);};},[active,doc,variant]);
 const stopFollowing=()=>setFollowing(false);
 const focus=activeLines[0]??(index<0&&doc?doc.lines.findIndex((_,i)=>visibleLyricLine(doc,i)):index);
 const pauseFocus=pause&&`pause-${pause.before}`;
 useLyricFollow(scroll,{active,following,doc,trackId:track?.id,variant,focus:pauseFocus??focus,last:pauseFocus??activeLines.at(-1)??focus,layoutEpoch,onPosition:()=>glowPosition.current()});
 const glow=useLyricGlow(pane,scroll,active&&current&&!!doc,`${track?.id}:${result?.revision}:${wordSignature}`,variant,doc);glowTick.current=glow.tick;glowPosition.current=glow.position;
 useLayoutEffect(()=>{updateClock.current?.();},[activeKey]);
 useLayoutEffect(()=>{
  if(!active||variant!=='full')return;
  const words=Array.from(scroll.current?.querySelectorAll<HTMLElement>('.lyric-word,.lyric-line-ink')??[]);
  const singleRun=words.map(word=>word.getClientRects().length===1);
  for(const [i,word] of words.entries()){
   if(singleRun[i])word.dataset.weightText=word.textContent??'';
   else delete word.dataset.weightText;
  }
  return()=>{for(const word of words)delete word.dataset.weightText;};
 },[active,variant,doc,layoutEpoch]);
 const pauseMarker=(before:number)=>{
  const item=pausesByLine.get(before);
  if(!item)return null;
  const shown=pause===item;
  return <div className="lyric-pause" data-line={`pause-${before}`} data-current={shown||undefined} aria-hidden={!shown}>
   {shown&&(variant==='full'?<div className="lyric-break"><span aria-hidden="true">Instrumental break</span><LyricActivity progressBar active={active} playing={player.is_playing} start={item.start} end={item.end}/></div>:<LyricActivity progressBar={variant==='visualizer'} active={active} playing={player.is_playing} start={item.start} end={item.end}/>)}
  </div>;
 };
 const state=doc?.metadata.instrumental?'Instrumental':doc&&!doc.lines.length?(doc.plain.trim()?'Plain lyrics':'No lyrics yet'):({offline:'You’re offline',not_found:'No lyrics found',missing:'No lyrics yet',choose:'Choose the right version',retry_later:'Lyrics provider is temporarily unavailable',provider_error:'Couldn’t load lyrics'} as Record<string,string>)[result?.state??''];
 const lyricActions:MenuItem[]=[{label:'Edit lyrics',icon:'Metadata',disabled:!track,onSelect:edit},{label:'Find lyrics',icon:'search',disabled:busy||retryIn>0||!track,onSelect:()=>void find()},{label:'Apple Music version',icon:'Lyrics',disabled:appleBusy||!track,onSelect:()=>void findApple()},{label:'Other versions',icon:'Lyrics',disabled:busy||retryIn>0||!track,onSelect:()=>void find(true)},...(result?.source?.url?[{label:`Source: ${result.source.label}`,icon:'Lyrics',onSelect:()=>void call('open_provider',{url:result.source!.url!})}]:[])];
 return <section ref={pane} className="lyrics-pane" data-current-cues={activeKey} data-variant={variant} data-following={following} data-lyric-pause={!!pause||undefined} aria-label="Lyrics" onContextMenu={e=>{e.preventDefault();e.stopPropagation();context({x:e.clientX,y:e.clientY});}} onKeyDown={e=>{if(e.key==='ContextMenu'||e.shiftKey&&e.key==='F10'){e.preventDefault();e.stopPropagation();context();}}} onPointerDown={e=>{if(e.pointerType==='touch'){const point={x:e.clientX,y:e.clientY};touchOrigin.current=point;longPress.current=window.setTimeout(()=>context(point),500);}}} onPointerUp={()=>clearTimeout(longPress.current)} onPointerCancel={()=>clearTimeout(longPress.current)} onPointerMove={e=>{if(Math.hypot(e.clientX-touchOrigin.current.x,e.clientY-touchOrigin.current.y)>6)clearTimeout(longPress.current);}}>
 <div className="lyrics-follow-slot" data-visible={!following&&!!doc?.lines.length&&current}>{variant==='mini'&&<span className="lyrics-mini-identification">{track?.title}</span>}<button className="lyrics-follow" disabled={following||!doc?.lines.length||!current} tabIndex={following?-1:0} aria-hidden={following||undefined} onClick={()=>{if(variant==='full')scroll.current?.focus({preventScroll:true});setFollowing(true);}}><Icon name="sync"/>Follow lyrics</button></div>
 {renderMenu?renderMenu(lyricActions):<Overflow contextOnly items={lyricActions}/>}

 {!current&&<div className="lyrics-preview-notice"><strong>{track?.title}</strong><button disabled={starting||!track?.available} onClick={()=>{setStarting(true);setPlayError('');void onPlay().catch(e=>setPlayError(errorText(e))).finally(()=>setStarting(false));}}><Icon name="play"/>{starting?'Starting…':'Play this song'}</button></div>}
 {appleBusy&&<div className="apple-lyric-preview" role="status"><LyricActivity loading active={active}/>Checking Apple Music…</div>}{appleError&&<p className="inline-error" role="alert">{appleError}</p>}{appleCandidate&&<div className="apple-lyric-preview"><span><strong>Apple Music version</strong><small>{appleCandidate.doc.lines.length} timed lines · {appleCandidate.doc.lines.some(l=>l.words?.length)?'Word synced':'Line synced'}</small></span><button disabled={appleBusy} onClick={()=>void selectApple()}>Use this version</button><button onClick={()=>setAppleCandidate(null)}>Cancel</button></div>}
 {busy&&!doc&&(!result||result.track_id!==track?.id)?<div className="lyrics-empty" role="status"><LyricActivity loading active={active}/><h2>Finding your words…</h2><p>Checking saved lyrics, connected Apple Music, and LRCLIB.</p></div>:doc&&!doc.metadata.instrumental&&(doc.lines.length>0||!!doc.plain.trim())?<div ref={scroll} className="lyrics-lines" tabIndex={0} aria-label="Song lyrics" onWheel={stopFollowing} onTouchStart={stopFollowing} onKeyDown={e=>{if(['ArrowUp','ArrowDown','PageUp','PageDown','Home','End'].includes(e.key))stopFollowing();}}>
 {doc.lines.length?<>{doc.lines.map((line,i)=>{const concealed=variant==='full'&&current&&following&&i>index&&i!==nextCue;return <Fragment key={`${line.start_ms}-${i}`}>{pauseMarker(i)}{visibleLyricLine(doc,i)&&<button data-line={i} data-concealed={concealed||undefined} aria-hidden={concealed||undefined} data-in-window={activeSet.has(i)||i===(pause?.before??(index<0?0:Math.max(index,...activeLines)+1))||undefined} style={lineStyle(i)} tabIndex={activeSet.has(i)||i===focus?0:-1} aria-label={`${lyricSpeaker(doc,line)}: ${line.text}`} data-voice={lyricVoice(line)} data-past={current&&i<=index&&!activeSet.has(i)||undefined} data-current={activeSet.has(i)||undefined} dir="auto" className="lyric-line" disabled={!current} aria-current={activeSet.has(i)?'true':undefined} onClick={()=>{setFollowing(true);void onSeek(Math.max(0,(line.start_ms+doc.metadata.offset_ms)/1000));}}>{wordSegments[i]?wordSegments[i]!.map((segment,n)=>segment.word===undefined?segment.text:<span key={`word-${n}`} className="lyric-word" data-word-start={line.words![segment.word].start_ms} data-word-end={lyricWordEnd(line,segment.word)} style={{'--word-progress':`${wordProgress(line.words![segment.word].start_ms,lyricWordEnd(line,segment.word),position*1000-doc.metadata.offset_ms)*100}%`,'--word-feather':lyricWordActive(line,segment.word,position*1000-doc.metadata.offset_ms)?'8%':'0%'} as import('react').CSSProperties} data-word-active={current&&activeSet.has(i)&&lyricWordActive(line,segment.word,position*1000-doc.metadata.offset_ms)||undefined} data-sung={current&&position*1000-doc.metadata.offset_ms>=line.words![segment.word].start_ms||undefined}>{segment.text}</span>):line.text.trim()&&!/^[\s♪♫♬♩]+$/.test(line.text)?(variant==='full'?line.text.split(/(\s+)/u).map((text,j)=>text.trim()?<span className="lyric-line-ink" key={j}>{text}</span>:text):line.text):<LyricActivity active={active&&i===index} playing={player.is_playing} start={line.start_ms+doc.metadata.offset_ms} end={(doc.lines[i+1]?.start_ms??track!.duration*1000)+doc.metadata.offset_ms}/>}</button>}</Fragment>;})}{pauseMarker(doc.lines.length)}</>:<div className="lyrics-plain" dir="auto">{doc.plain}</div>}
 </div>:<div className="lyrics-empty"><Icon name="Lyrics"/><h2>{track?(state||'Lyrics'):'Nothing playing yet'}</h2><p>{!track?'Choose a song in Library.':retryIn?`Retry available in ${retryIn}s.`:result?.state==='retry_later'?'LRCLIB is busy. This doesn’t mean the song has no lyrics.':result?.message??(doc?.metadata.instrumental?'Let the music speak.':'Find another version, or add your own lyrics.')}</p>{variant!=='mini'&&track&&['retry_later','offline','provider_error'].includes(result?.state??'')&&<button className="lyrics-retry" disabled={busy||retryIn>0} onClick={()=>void find()}><Icon name="sync"/>{busy?'Finding…':retryIn?`Retry in ${retryIn}s`:'Retry lyrics'}</button>}</div>}
 {state&&doc&&<small className="lyrics-kind">{state}</small>}
 {(error||playError)&&<p className="inline-error" role="alert">{error||playError}</p>}
 {!!result?.candidates.length&&!doc&&<div className="lyrics-candidates">{result.candidates.map(c=><button key={c.id} disabled={busy} onClick={()=>void select(c.id)}><span><strong>{c.title}</strong><small>{c.artist} · {c.album} · {Math.round(c.duration)} s</small></span><small>{c.document.lines.some(l=>l.words?.length)?'Word synced':c.document.lines.length?'Line synced':c.document.metadata.instrumental?'Instrumental':'Plain'}</small></button>)}</div>}
 </section>;
}
