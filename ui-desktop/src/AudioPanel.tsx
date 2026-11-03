import {haptic} from './haptics';
import {KeepView} from './ViewScope';
import {usePlayerHandoff} from './usePlayerHandoff';
import {useEffect,useLayoutEffect,useRef,useState,type CSSProperties} from 'react';
import {Cover,Icon,QueueRows,duration} from './components';
import LyricsView from './LyricsView';
import ThemedSwitch from './ThemedSwitch';
import ListeningPanelHeader from './ListeningPanelHeader';
import {useViewActive} from './ViewScope';

import {songDragFeedback,type SongDrag} from './songDrag';
import type {Playback,PlaybackAction} from './types';
export default function AudioPanel({player,scale,presentation,closing=false,onClose,onAction,onSettings,theme,view='queue'}:{view?:'queue'|'lyrics';theme?:CSSProperties;player:Playback;scale:number;closing?:boolean;presentation:'sidebar'|'floating'|'embedded';onClose:()=>void;onAction:(action:PlaybackAction)=>void|Promise<void>;onSettings:()=>void}){
 const active=useViewActive();
 const offset=player.queue_index===null?0:player.queue_index+1,upcoming=player.queue.slice(offset);
 const panel=useRef<HTMLElement>(null),opener=useRef(document.querySelector<HTMLElement>('[aria-label="Open Up next"]')??document.activeElement as HTMLElement|null);
 usePlayerHandoff(panel,player.track?.id,view,player.queue_entry_ids?.[player.queue_index??-1]);
 const [docked,setDocked]=useState(false),[songDrop,setSongDrop]=useState(false);
 const actionRef=useRef(onAction);actionRef.current=onAction;
 const playerRef=useRef(player);playerRef.current=player;
 useEffect(()=>{if(!active||view!=='queue'){setSongDrop(false);return;}let frame=0,last:SongDrag|null=null;
 const clear=()=>{panel.current?.querySelectorAll('[data-incoming-drop]').forEach(row=>row.removeAttribute('data-incoming-drop'));};
 const locate=(drag:SongDrag)=>{clear();const rows=[...(panel.current?.querySelectorAll<HTMLElement>('[data-queue-index]')??[])];let index=playerRef.current.queue.length;
 for(const row of rows){const box=row.getBoundingClientRect();if(drag.y<box.top+box.height/2){index=(playerRef.current.queue_index??-1)+1+Number(row.dataset.queueIndex);row.setAttribute('data-incoming-drop','true');break;}}return index;};
 const scroll=()=>{if(!last)return;const list=panel.current?.querySelector<HTMLElement>('.queue-scroll');if(list){const r=list.getBoundingClientRect();const speed=last.y<r.top+32?-8:last.y>r.bottom-32?8:0;if(speed){list.scrollTop+=speed;locate(last);}}frame=requestAnimationFrame(scroll);};
 const update=(event:Event)=>{const drag=(event as CustomEvent<SongDrag>).detail,box=panel.current?.getBoundingClientRect();const inside=!!box&&drag.x>=box.left&&drag.x<=box.right&&drag.y>=box.top&&drag.y<=box.bottom;setSongDrop(!drag.drop&&inside);
 if(inside&&!drag.drop){songDragFeedback(`Add ${(drag.ids??[drag.id]).length} ${(drag.ids??[drag.id]).length===1?'song':'songs'} to Up next`);last=drag;locate(drag);if(!frame)frame=requestAnimationFrame(scroll);}else{last=null;cancelAnimationFrame(frame);frame=0;}
 if(drag.drop&&!drag.cancelled&&inside){const index=locate(drag),ids=[...new Set(drag.ids??[drag.id])];void(async()=>{for(let offset=0;offset<ids.length;offset++)await actionRef.current({type:'add_to_queue',id:ids[offset],index:index+offset});haptic('drop',`queue:${index}`);})();}if(drag.drop||!inside)clear();};
 window.addEventListener('musicos-song-drag',update);return()=>{cancelAnimationFrame(frame);clear();window.removeEventListener('musicos-song-drag',update);};},[active,view]);
 const state=useRef({docked,onClose,closing});state.current={docked,onClose,closing};
 useLayoutEffect(()=>{if(presentation==='embedded')return;const shell=panel.current?.closest<HTMLElement>('.app-shell');if(!shell)return;const header=shell.querySelector<HTMLElement>('.app-header');const update=()=>{const font=parseFloat(getComputedStyle(shell).fontSize);const sidebar=shell.querySelector('.library-sidebar')?.getBoundingClientRect().width??0;const canDock=presentation==='sidebar'&&shell.clientWidth-sidebar>=50*font;setDocked(canDock);shell.dataset.audioSidebar=String(canDock);shell.style.setProperty('--audio-top',`${shell.querySelector('.workspace')?.getBoundingClientRect().top??header?.offsetHeight??64}px`);};const observer=new ResizeObserver(update);observer.observe(shell);if(header)observer.observe(header);update();return()=>{observer.disconnect();delete shell.dataset.audioSidebar;};},[presentation,scale]);
 // Wait for a completed click: pointer-down may begin an internal song drag
 // into this floating panel. The drag suppresses its click after threshold.
 useEffect(()=>{if(presentation==='embedded')return;const node=panel.current;const escape=(e:KeyboardEvent)=>{if(!state.current.closing&&e.key==='Escape'&&!e.defaultPrevented){e.preventDefault();state.current.onClose();}};const outside=(e:MouseEvent)=>{if(!state.current.closing&&!e.defaultPrevented&&!state.current.docked&&!panel.current?.contains(e.target as Node)&&!(e.target as Element).closest('[data-player-cycle],[aria-label="Open Up next"],[role=menu]'))state.current.onClose();};window.addEventListener('keydown',escape);window.addEventListener('click',outside);return()=>{window.removeEventListener('keydown',escape);window.removeEventListener('click',outside);if(opener.current?.isConnected&&(document.activeElement===document.body||node?.contains(document.activeElement)))opener.current.focus();};},[]);
 return <aside style={theme} data-embedded={presentation==='embedded'||undefined} data-closing={closing} aria-hidden={closing} inert={closing} data-song-drop={songDrop} ref={panel} className={`audio-panel queue-panel ${docked?'audio-docked':'audio-floating'}`} aria-label="Up next"><ListeningPanelHeader title={view==='lyrics'?'Lyrics':'Up next'} closeLabel="Close Up next" onClose={onClose} track={player.track}/>

 <KeepView className="retained-queue" active={view==='queue'}><div className="queue-dj"><Icon name="DJmode"/><ThemedSwitch label="DJmode" checked={!!player.dj_mode} onChange={enabled=>onAction({type:"dj_mode",enabled})}/>{player.dj_transition?.status==='active'&&<small role="status">Mixing</small>}</div>
 <div className="audio-up-next"><small>{upcoming.length} songs · {duration(upcoming.reduce((sum,t)=>sum+t.duration,0))}</small>{upcoming.length>0&&<button onClick={()=>onAction({type:'clear_upcoming'})}>Clear upcoming</button>}</div>
 {upcoming.length?<QueueRows tracks={upcoming} entryIds={player.queue_entry_ids?.slice(offset)} arranged={player.arranged_entries} pinned={player.pinned_entries} current={null} scale={scale} onSelect={index=>onAction({type:'select',index:index+offset})} onRemove={index=>onAction({type:'remove',index:index+offset})} onMove={(from,to)=>onAction({type:'move',from:from+offset,to:to+offset})}/>:<div className="queue-empty"><Icon name="Song"/><strong>Your next listen starts here</strong><p>Play an album or add songs to Up next.</p></div>}</KeepView><KeepView className="retained-panel-lyrics" active={view==='lyrics'}><LyricsView variant="panel" track={player.track??undefined} player={player} active={active&&!closing} onSeek={async position=>{await onAction({type:'seek',position});}} onPlay={async()=>{}}/></KeepView>
 </aside>;
}
