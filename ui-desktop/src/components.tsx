import {haptic} from './haptics';
import {useMenuPresentation,menuOwnsFocus} from './menuPresentation';
import {flushSync} from 'react-dom';
import {usePlaybackClock} from './playbackFeed';
import {useContext} from 'react';
import {TrackSelection} from './TrackSelection';
import {
  useCallback,
  useLayoutEffect,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type RefObject,
  type ReactNode,
} from "react";
import * as Dropdown from "@radix-ui/react-dropdown-menu";
import { useVirtualizer } from "@tanstack/react-virtual";
import { call, artwork, asset, native, localArtwork, libraryArtwork } from "./bridge";
import type { QueueMix, Track } from "./types";
import { AssetIcon, portraitPlaceholder } from "./AssetIcon";
import FavoriteButton from './FavoriteButton';
import {trackCredits} from './credits';
import {useArtworkParallax} from './motion';
import {useDepthArtwork} from './depthArtwork';
import {startSongDrag} from './songDrag';
import { useGridReflow } from "./motion";
import {usePortraitArtwork} from './portraitArtwork';
import {albumArtwork} from './albumArtwork';
import {useArtworkImage} from './useArtworkImage';
import {useArtworkRendition} from './useArtworkRendition';
export const Icon = AssetIcon;
export function Cover({
  src,
  trackId,
  fallback,
  title,
  portrait = false,
  onLoadingChange,
  songArtwork = false,
}: {
  songArtwork?:boolean;
  src?: string | null;
  trackId?: number;
  fallback?: string | null;
  title: string;
  portrait?: boolean;
  onLoadingChange?:(loading:boolean)=>void;
}) {
  const [epoch, setEpoch] = useState(0);
  const [extracted, setExtracted] = useState<{
    id: number;
    path: string | null;
  } | null>(null);
  useEffect(() => {
    const refresh = () => {
      setEpoch((v) => v + 1);
    };
    window.addEventListener("musicos-artwork-invalidated", refresh);
    return () =>
      window.removeEventListener("musicos-artwork-invalidated", refresh);
  }, []);
  const imageRef = useRef<HTMLImageElement>(null);
  const [failed, setFailed] = useState<string[]>([]);
  const [cached, setCached] = useState<{ url: string; path: string } | null>(
    null,
  );
  const canonical=portrait?undefined:albumArtwork(trackId);
  const recovered=extracted&&extracted.id===trackId&&extracted.path&&!failed.includes(extracted.path)?extracted.path:null;
  const candidate=canonical!==undefined
    ? canonical&&!failed.includes(canonical)?canonical:recovered
    : songArtwork&&recovered?recovered
      : src&&!failed.includes(src)?src
        : fallback&&!failed.includes(fallback)?fallback:recovered;
  useEffect(() => {
    // A known missing canonical cover must still reach recovery. Its stale src
    // is not a usable image, and must not block the visible-album extraction.
    if (!native || !trackId || !imageRef.current || (canonical ? !failed.includes(canonical) : canonical===undefined&&!songArtwork&&src&&!failed.includes(src))) return;
    let gone = false;
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        observer.disconnect();
        void libraryArtwork(trackId,songArtwork)
          .then((path) => {
            if (!gone) { setExtracted({ id: trackId, path }); if(path)setFailed(old=>old.includes(path)?old.filter(value=>value!==path):old); }
          })
          .catch(() => {});
      },
      { rootMargin: "200px" },
    );
    observer.observe(imageRef.current);
    return () => {
      gone = true;
      observer.disconnect();
    };
  }, [src, trackId, epoch, failed, songArtwork, canonical]);
  const remote = !!candidate?.startsWith("https://") && native;
  const resolved = remote
    ? cached?.url === candidate
      ? cached.path
      : null
    : candidate;
  useEffect(() => {
    setFailed(old=>old.length?[]:old);
  }, [src, fallback]);
  useEffect(() => {
    if (!remote || !candidate || !imageRef.current) return;
    let gone = false;
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        observer.disconnect();
        void localArtwork(candidate)
          .then((path) => {
            if (!gone) setCached({ url: candidate, path });
          })
          .catch(() => {
            if (!gone) setFailed((old) => [...old, candidate]);
          });
      },
      { rootMargin: "200px" },
    );
    observer.observe(imageRef.current);
    return () => {
      gone = true;
      observer.disconnect();
    };
  }, [remote, candidate]);
  const markFailed=()=>{if(candidate)setFailed(old=>old.includes(candidate)?old:[...old,candidate]);};
  const rendition=useArtworkRendition(portrait?null:resolved,epoch,imageRef,markFailed);
  const framed=usePortraitArtwork(portrait?resolved:null,epoch);
  const displayPath=portrait?framed?.display:rendition;
  const pending=!!resolved&&!displayPath;
  const desiredUrl=displayPath?artwork(displayPath)+(native&&extracted?.path===resolved?`?musicos-artwork=${epoch}`:''):portrait?portraitPlaceholder:asset('assets/icons/ui/noCOVER.svg');
  const displayed=useArtworkImage({url:desiredUrl,path:displayPath?(portrait?framed?.path:resolved??undefined):undefined,original:resolved??undefined,version:epoch},`${portrait?'portrait':'cover'}:${trackId??title}`,!!resolved,markFailed,pending);
  useLayoutEffect(()=>{const image=imageRef.current;onLoadingChange?.(!!resolved&&!(image?.complete&&image.naturalWidth>0));},[resolved,onLoadingChange]);
  return (
    <img
      ref={imageRef}
      data-artwork-path={displayed.path}
      data-artwork-version={displayed.version}
      data-original-artwork-path={displayed.original}
      data-full-src={displayed.original?artwork(displayed.original):undefined}
      className={`cover ${portrait ? "portrait" : ""} ${!displayed.path ? "placeholder" : ""}`}
      src={displayed.url}
      alt={`${title} ${portrait ? "portrait" : "cover"}`}
      loading="lazy"
      onLoad={()=>onLoadingChange?.(false)}
      onError={() => {
        onLoadingChange?.(false);
        if (candidate && resolved && displayed.path && displayed.original===resolved) setFailed((old) => [...old, candidate]);
      }}
      draggable={false}
    />
  );
}
export function PortraitLoading(){return <span className="portrait-loading-symbol" role="status" aria-label="Loading artist photo"><svg viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true"><rect x="5" y="5" width="10" height="10" rx="2"/><rect x="17" y="17" width="10" height="10" rx="2"/></svg></span>;}
export function IconButton({
  name,
  label,
  onClick,
  disabled = false,
  pressed,
}: {
  name: string;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  pressed?: boolean;
}) {
  return (
    <button
      className="icon-button"
      title={label}
      aria-label={label}
      aria-pressed={pressed}
      onClick={e=>{if((name==='next'||name==='prev')&&!matchMedia('(prefers-reduced-motion: reduce)').matches){const glyph=e.currentTarget.querySelector<HTMLElement>('.icon');if(glyph){const from=getComputedStyle(glyph).transform;glyph.getAnimations().filter(a=>a.id==='transport-skip').forEach(a=>a.cancel());const direction=name==='next'?1:-1;const a=glyph.animate([{transform:from},{transform:`translateX(${direction*4}px)`,offset:.45},{transform:'translateX(0)'}],{duration:180,easing:'ease-out'});a.id='transport-skip';void a.finished.then(()=>a.cancel()).catch(()=>{});}}onClick();}}
      disabled={disabled}
    >
      <Icon name={name} className="control-glyph" />
    </button>
  );
}
function LiveMeter({children,trackId}:{children?:ReactNode;trackId?:number}){const clock=usePlaybackClock();const levels=trackId===undefined||clock.trackId===trackId?clock.levels:[];return <span className="live-meter" style={{'--level-1':levels[0]??0,'--level-2':levels[1]??0,'--level-3':levels[2]??0} as CSSProperties}>{children??<span className="equalizer" data-active="true">{[0,1,2].map(i=><i key={i} style={{transform:`scaleY(${levels[i]??0})`}}/>)}</span>}</span>;}
function SongMeterFeed({host,trackId}:{host:RefObject<HTMLSpanElement|null>;trackId:number}){
 const clock=usePlaybackClock();
 useLayoutEffect(()=>{const node=host.current;if(!node)return;const levels=clock.trackId===trackId?clock.levels:[];for(let i=0;i<3;i++)node.style.setProperty(`--level-${i+1}`,String(levels[i]??0));},[clock.levels,clock.trackId,trackId,host]);
 useEffect(()=>()=>{for(let i=0;i<3;i++)host.current?.style.removeProperty(`--level-${i+1}`);},[host]);return null;
}
// Only the current song subscribes to the output meter. Pointer intent is local
// to this control; moving the title never changes the target of an existing press.
function SongPlaybackControl({track,current,playing,levels,onPlay,onToggle}:{track:Track;current:boolean;playing:boolean;levels?:number[];onPlay:(track:Track)=>void;onToggle?:()=>void}){
 const button=useRef<HTMLButtonElement>(null),meterHost=useRef<HTMLSpanElement>(null);
 const [engaged,setEngaged]=useState(false);
 useEffect(()=>{
  const row=button.current?.closest<HTMLElement>('.track-row');if(!row)return;
  let hovered=row.matches(':hover');
  const update=()=>{if(!row.dataset.songDragging)setEngaged(hovered||row.matches(':focus-visible')||!!row.querySelector(':focus-visible'));};
  const enter=()=>{hovered=true;update();},leave=()=>{hovered=false;update();};
  const focusOut=()=>queueMicrotask(update);
  row.addEventListener('pointerenter',enter);row.addEventListener('pointerleave',leave);row.addEventListener('focusin',update);row.addEventListener('focusout',focusOut);row.addEventListener('musicos-song-drag-end',update);update();
  return()=>{row.removeEventListener('pointerenter',enter);row.removeEventListener('pointerleave',leave);row.removeEventListener('focusin',update);row.removeEventListener('focusout',focusOut);row.removeEventListener('musicos-song-drag-end',update);};
 },[track.id]);
 const meter=current&&playing;
 const glyph=<Icon name={meter?(engaged?'pause':'rowLevels'):'play'}/>;
 return <button ref={button} className="row-play" disabled={!track.available} aria-label={`${meter?'Pause':'Play'} ${track.title}`} data-meter={meter} style={{'--level-1':levels?.[0]??0,'--level-2':levels?.[1]??0,'--level-3':levels?.[2]??0} as CSSProperties} onClick={e=>{e.stopPropagation();if(current&&onToggle)onToggle();else onPlay(track);}}><span className="live-meter" ref={meterHost}>{meter&&<SongMeterFeed host={meterHost} trackId={track.id}/>} {glyph}</span></button>;
}
export interface MenuItem {
  label: string;
  onSelect: () => void;
  icon?: string;
  disabled?: boolean;
  portrait?: string|null;
  group?: "playback"|"organize"|"danger";
}
export function Menu({
  label,
  children,
  items,
  contextOnly = false,
  selectedLabel,
  onIntent,
  theme,
  morph,
  triggerClassName="",
}: {
  contextOnly?: boolean;
  triggerClassName?:string;
  morph?:"dots"|"plus";
  theme?:CSSProperties;
  selectedLabel?:string;
  onIntent?:()=>void;
  label: string;
  children: ReactNode;
  items: MenuItem[];
}) {
  const [inherited,setInherited]=useState<CSSProperties>({});
  const [point,setPoint]=useState<{x:number;y:number}|null>(null);
  const [buttonSide,setButtonSide]=useState<'top'|'bottom'>('bottom');
  const trigger=useRef<HTMLButtonElement>(null),origin=useRef<HTMLElement|null>(null),contextOpened=useRef(false),didSelect=useRef(false);
  const [portalContainer,setPortalContainer]=useState<HTMLElement|undefined>();
  useLayoutEffect(()=>setPortalContainer(trigger.current?.closest('dialog')??undefined),[]);
  const panel=useMenuPresentation(trigger);
  const {open,present,setOpen,content}=panel;
  const ownerTheme=()=>{const node=trigger.current;if(!node)return;origin.current=node.closest<HTMLElement>('.lyrics-pane')??node.closest<HTMLElement>('[data-action-surface],.track-row,.queue-row,.collection-card')?.querySelector<HTMLElement>('button:not(.context-only)')??node;const style=getComputedStyle(node),values:Record<string,string>={};for(const key of ['--surface-color','--control-surface','--control-border','--ink','--secondary-ink','--accent','--accent-ink','--collection-hue-1','--collection-hue-2'])values[key]=style.getPropertyValue(key);setInherited(values as CSSProperties);};
  const group=(item:MenuItem)=>item.group??(/delete|remove|forget/i.test(item.label)?'danger':/^(play|shuffle|add to queue)/i.test(item.label)?'playback':'organize');
  const currentItems=selectedLabel?items.filter(item=>item.label!==selectedLabel):[...items].sort((a,b)=>['playback','organize','danger'].indexOf(group(a))-['playback','organize','danger'].indexOf(group(b)));
  const retainedItems=useRef(currentItems);
  if(open)retainedItems.current=currentItems;
  // Navigation may change the available choices during exit. Keep the outgoing
  // composition until it is gone, rather than swapping labels mid-collapse.
  const ordered=present&&!open?retainedItems.current:currentItems;

  useEffect(()=>()=>{trigger.current?.removeAttribute('data-connected-side');},[]);
  const position=(menu:HTMLDivElement)=>{if(selectedLabel&&trigger.current){const rect=trigger.current.getBoundingClientRect();menu.style.left=`${rect.left}px`;menu.style.top=`${rect.bottom-1}px`;menu.style.width=`${rect.width}px`;menu.style.maxHeight=`${innerHeight-rect.bottom-12}px`;return;}if(!point){for(const key of ['left','top','--menu-origin'])menu.style.removeProperty(key);const rect=trigger.current?.getBoundingClientRect(),list=menu.querySelector<HTMLElement>('.menu-panel-items');if(!rect||!list)return;
    // Radix only flips a panel that fits on the other side. Pick the side with
    // room for every item, else the roomier one, and scroll within that space.
    const below=innerHeight-rect.bottom-14,above=rect.top-14,natural=list.scrollHeight+menu.offsetHeight-menu.clientHeight,next=natural<=below||natural>above&&below>=above?'bottom':'top';menu.style.maxHeight=`min(24rem,${Math.max(0,Math.floor(next==='bottom'?below:above))}px)`;setButtonSide(next);return;}menu.style.removeProperty('max-height');const y=point.y;const above=y+menu.offsetHeight>innerHeight-8;const side=above?'top':'bottom';if(menu.dataset.side!==side)menu.dataset.side=side;menu.style.left=`${Math.max(8,Math.min(point.x,innerWidth-menu.offsetWidth-8))}px`;menu.style.top=`${Math.max(8,Math.min(above?point.y-menu.offsetHeight:y,innerHeight-menu.offsetHeight-8))}px`;menu.style.setProperty('--menu-origin',`${Math.max(0,Math.min(menu.offsetWidth,point.x-parseFloat(menu.style.left)))}px ${Math.max(0,Math.min(menu.offsetHeight,point.y-parseFloat(menu.style.top)))}px`);};
  useLayoutEffect(()=>{if(!open||!content.current)return;const menu=content.current;position(menu);const observer=new MutationObserver(()=>{position(menu);});observer.observe(menu,{attributes:true,attributeFilter:['data-side']});const frame=requestAnimationFrame(()=>{position(menu);});return()=>{cancelAnimationFrame(frame);observer.disconnect();};},[open,point,panel.element]);
  useEffect(()=>{if(!open)return;const update=()=>{if(content.current){position(content.current);}};window.addEventListener('resize',update);window.addEventListener('scroll',update,true);return()=>{window.removeEventListener('resize',update);window.removeEventListener('scroll',update,true);};},[open,point]);
  useEffect(()=>{const button=trigger.current;const show=(event:Event)=>{const at=(event as CustomEvent<{x:number;y:number}>).detail??null;didSelect.current=false;contextOpened.current=true;ownerTheme();setPoint(at);setOpen(true);};button?.addEventListener('musicos-open-menu',show);return()=>button?.removeEventListener('musicos-open-menu',show);},[setOpen]);
  return (
    <Dropdown.Root modal={false} open={open} onOpenChange={next=>{if(next){setPoint(null);didSelect.current=false;contextOpened.current=false;ownerTheme();}setOpen(next);}}>

      <Dropdown.Trigger asChild>
        <button onPointerEnter={onIntent} onFocus={onIntent} ref={trigger} className={`menu-trigger ${triggerClassName} ${morph==="dots"?"soft-action":""} ${contextOnly?"context-only":""}`} data-morph={morph} data-panel-present={present||undefined} tabIndex={contextOnly?-1:undefined} aria-hidden={contextOnly||undefined} title={label} aria-label={label} data-connected-open={open&&!selectedLabel||undefined}>
          {morph?<><Icon name={morph==='plus'?(open?'menuClose':'menuPlus'):(open?'menuDotsClose':'3dots_(nocircle)')}/>{morph==='dots'&&<span>More</span>}</>:children}
        </button>
      </Dropdown.Trigger>
      {present&&<Dropdown.Portal container={portalContainer} forceMount>
        <Dropdown.Content
          forceMount
          ref={panel.ref}
          // Section chrome follows the live app palette, including artwork that
          // finishes decoding during the reveal. Item menus keep their owner snapshot.
          style={selectedLabel?theme:{...inherited,...theme}}
          inert={!open||undefined}
          aria-hidden={!open||undefined}
          data-pointer-menu={point?true:undefined}
          className={`dropdown menu-panel ${selectedLabel?"section-dropdown":""}`}
          align={selectedLabel?"end":"start"}
          side={selectedLabel||point?"bottom":buttonSide}
          sideOffset={selectedLabel?0:6}
          collisionPadding={selectedLabel?0:8}
          onEscapeKeyDown={panel.onEscapeKeyDown}
          onInteractOutside={event=>{const target=event.detail.originalEvent.target;if(!open||target instanceof Node&&trigger.current?.contains(target))event.preventDefault();}}
          onCloseAutoFocus={event=>{event.preventDefault();if(didSelect.current||!menuOwnsFocus(trigger.current,panel.session)){didSelect.current=false;contextOpened.current=false;return;}const target=contextOpened.current||contextOnly?origin.current:trigger.current;if(target?.isConnected)target.focus({preventScroll:true});contextOpened.current=false;}}

        >
          <div className="menu-panel-surface" aria-hidden="true"/><div className="menu-panel-items">
          {ordered.map((item, i) => (
            <Dropdown.Item
              className="dropdown-item"
              data-group-start={i>0&&group(item)!==group(ordered[i-1])||undefined}
              key={`${item.label}-${i}`}
              onSelect={event=>{event.preventDefault();didSelect.current=true;const target=contextOpened.current||contextOnly?origin.current:trigger.current;flushSync(()=>{if(selectedLabel)panel.closeThen(item.onSelect);else setOpen(false);});if(target?.isConnected)target.focus({preventScroll:true});if(!selectedLabel)item.onSelect();}}
              disabled={item.disabled}
            >
              {item.portrait?<Cover src={item.portrait} title="" portrait/>:item.icon&&<Icon name={item.icon}/>}
              <span>{item.label}</span>
            </Dropdown.Item>
          ))}
          </div>
        </Dropdown.Content>
      </Dropdown.Portal>}
    </Dropdown.Root>
  );
}
export function Overflow({ items, contextOnly=false,theme }: { items: MenuItem[]; contextOnly?:boolean;theme?:CSSProperties }) {
  return (
    <Menu label="More actions" items={items} contextOnly={contextOnly} theme={theme}>
      <Icon name="3dots_(nocircle)" />
    </Menu>
  );
}
export function duration(seconds: number) {
  const n = Math.max(0, Math.floor(seconds));
  return `${Math.floor(n / 60)}:${String(n % 60).padStart(2, "0")}`;
}
export function collectionDuration(seconds: number) {
  if (seconds < 60) return `${Math.max(0, Math.round(seconds))} sec`;
  return seconds >= 3600
    ? `${Math.floor(seconds / 3600)} hr ${Math.round((seconds % 3600) / 60)} min`
    : `${Math.round(seconds / 60)} min`;
}
export function Empty({
  title,
  children,
  action,
}: {
  title: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <section className="empty">
      <Icon name="Album" />
      <h2>{title}</h2>
      <p>{children}</p>
      {action}
    </section>
  );
}
export function TrackRows({
  tracks,
  scrollRef,
  scale,
  initialOffset = 0,
  onPlay,
  onToggle,
  onArtist,
  onAlbum,
  onEdit,
  onFavorite,
  playlistActions,
  playingId,
  tempos,
  isPlaying = false,
  selected,
  onSelect,
  albumArtist,showDurations=true,levels,
}: {
  albumArtist?:string;showDurations?:boolean;levels?:number[];
  tracks: Track[];
  scrollRef: RefObject<HTMLElement | null>;
  scale: number;
  initialOffset?: number;
  onPlay: (track: Track) => void;
  onToggle?: () => void;
  onArtist: (track: Track, name?:string) => void;
  onAlbum?: (track: Track) => void;
  onEdit?: (track: Track,ids?:number[]) => void;
  onFavorite?: (track: Track) => void;
  playlistActions?: (track: Track,ids:number[]) => MenuItem[];
  playingId?: number;
  tempos?:ReadonlyMap<number,number>;
  isPlaying?: boolean;
  selected?: string;
  onSelect: (id: string) => void;
}) {
  const selection=useContext(TrackSelection);
  const container = useRef<HTMLDivElement>(null);
  const [margin, setMargin] = useState(0);
  useLayoutEffect(() => {
    if (container.current && scrollRef.current)
      setMargin(
        container.current.getBoundingClientRect().top -
          scrollRef.current.getBoundingClientRect().top +
          scrollRef.current.scrollTop,
      );
  }, [tracks, scale, scrollRef]);
  const multiDisc=!!albumArtist&&new Set(tracks.map(t=>Math.max(1,t.disc_number))).size>1;
  const entries:Array<{track?:Track;index:number;disc?:number}>=[];
  tracks.forEach((track,index)=>{const disc=Math.max(1,track.disc_number);if(multiDisc&&(index===0||Math.max(1,tracks[index-1].disc_number)!==disc))entries.push({index,disc});entries.push({track,index});});
  const virtual = useVirtualizer({useAnimationFrameWithResizeObserver:true,enabled:tracks.length>70,initialOffset,count:entries.length,getItemKey:i=>entries[i].track?.id??`disc-${entries[i].disc}`,getScrollElement:()=>scrollRef.current,estimateSize:i=>(entries[i].disc?38:44)*scale,gap:4*scale,overscan:8,scrollMargin:margin});
  const rows=tracks.length>70?virtual.getVirtualItems().map(v=>({...entries[v.index],virtualIndex:v.index,style:{position:'absolute',top:0,left:0,width:'100%',transform:`translateY(${v.start-margin}px)`} as CSSProperties})):entries.map((entry,virtualIndex)=>({...entry,virtualIndex,style:undefined}));
  return (
    <div className="track-table" data-local-songs data-durations={showDurations} role="table" aria-label="Songs">
      <div className="track-head" role="row">
        <span>#</span>
        <span>Title / Artist</span>
        <span className="track-album">Album</span>
        <span>Time</span>
        <span className="track-actions-heading" aria-label="Song actions" />
      </div>
      <div
        ref={container}
        className="track-body"
        data-virtual={tracks.length>70}
        style={
          tracks.length > 70
            ? { height: virtual.getTotalSize(), position: "relative" }
            : undefined
        }
      >
        {rows.map(({track,index,style,disc,virtualIndex}) => !track?<div key={`disc-${disc}`} ref={tracks.length>70?virtual.measureElement:undefined} data-index={virtualIndex} style={style} className="disc-heading" role="row"><span role="cell">Disc {disc}</span></div>:( 
          <div
            role="row"
            data-track-id={track.id}
            data-index={virtualIndex}
            ref={tracks.length>70?virtual.measureElement:undefined}
            className={`track-row ${playingId === track.id ? "playing" : ""} ${(selection?selection.ids.has(track.id):selected === String(track.id)) ? "selected" : ""}`}
            style={style}
            key={track.id}
            onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}}
            onKeyDown={e=>{if(selection&&e.target===e.currentTarget){if((e.metaKey||e.ctrlKey)&&e.key==='a'){e.preventDefault();selection.selectAll?.();}if(e.key===' '){e.preventDefault();selection.select(track.id,index,e as unknown as React.MouseEvent);}if(e.key==='Delete'||e.key==='Backspace'){e.preventDefault();selection.remove();}if(e.key==='Enter'&&track.available){e.preventDefault();onPlay(track);}}if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}}
            tabIndex={selection?0:undefined}
            aria-selected={selection?.ids.has(track.id)}
            onClick={e=>{if((e.target as Element).closest("button,a,input"))return;if(selection){e.currentTarget.focus({preventScroll:true});selection.select(track.id,index,e);}else if(!selection)onSelect(String(track.id));}}
            onPointerDown={e=>{if(track.available&&!(e.target as Element).closest("button,a,input"))startSongDrag(e,track,selection?.ids.has(track.id)?tracks.filter(t=>selection.ids.has(t.id)).map(t=>t.id):[track.id]);}}
            onDoubleClick={e=>{if(track.available&&!(e.target as Element).closest("button,a,input"))onPlay(track);}}
          >
            <span className="track-index" role="cell">
              <span>{multiDisc?track.track_number||index+1:index+1}</span>
            </span>
            <div className="track-title" role="cell">
              <SongPlaybackControl track={track} current={playingId===track.id} playing={isPlaying} levels={levels} onPlay={onPlay} onToggle={onToggle}/>
              <div className="track-label">
                <span title={track.title}>{track.title}</span>
                <div className="track-credits" title={track.artist} hidden={!!albumArtist&&trackCredits(track).length===1&&trackCredits(track)[0].toLocaleLowerCase()===albumArtist.trim().toLocaleLowerCase()}>{trackCredits(track).map((name,index)=><span key={name}>{index>0?(trackCredits(track).length===2?' & ':', '):''}<button className="text-link muted" onClick={e=>{e.stopPropagation();onArtist(track,name);}}>{name}</button></span>)}{!track.available?' · Unavailable':''}</div>
              </div>
            </div>
            <button
              className="track-album text-link"
              role="cell"
              title={track.album}
              onClick={(e) => {
                e.stopPropagation();
                onAlbum?.(track);
              }}
            >
              {track.album || "Unknown album"}
            </button>
            <span className="time" role="cell">
              {duration(track.duration)}
            </span>
            <div className="track-actions">{track.audio_quality&&<span className="track-quality" title={`${track.audio_quality.format} · ${track.audio_quality.lossless?'Lossless':'Lossy'}`}><Icon name={track.audio_quality.lossless?'Lossless':'Lossy'}/><span className="sr-only">{track.audio_quality.lossless?'Lossless':'Lossy'}</span></span>}
            <FavoriteButton track={track} bpm={tempos?.get(track.id)} playing={playingId===track.id&&isPlaying} onFavorite={onFavorite}/>
            <Overflow
              items={[...(playlistActions?.(track,selection?.ids.has(track.id)?[...selection.ids]:[track.id])??[]),...(onEdit?[{label:"Edit metadata",icon:"Metadata",onSelect:()=>onEdit(track,selection?.ids.has(track.id)?[...selection.ids]:[track.id])}]:[]),...(selection?.canRemove?[{label:"Remove from playlist",icon:"Delete",onSelect:()=>selection.remove(selection.ids.has(track.id)?[...selection.ids]:[track.id])}]:[])]}
            />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
export interface CollectionItem {
  id: string;
  title: string;
  subtitle: string;
  cover?: string | null;
  trackId?: number;
  portrait?: boolean;
  icon?: string;
  onClick: (event?:React.MouseEvent<HTMLElement>) => void;
  menu?: MenuItem[];
  levels?:number[];
  playing?:boolean;
  isPlaying?:boolean;
  loading?:boolean;
  loadingCover?:string|null;
}
function Card({
  item,
  selected,
  onSelect,
}: {
  item: CollectionItem;
  selected?: string;
  onSelect: (id: string) => void;
}) {
  const photoPending=!!item.portrait&&!item.cover&&!!item.loading;
  return (
    <article
      className={`collection-card ${item.playing ? "currently-playing" : ""}`}
      data-collection-id={item.id}
      data-artist-name={item.portrait?item.title:undefined}
      onContextMenu={e=>{if(item.menu){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}}}
      onKeyDown={e=>{if(item.menu&&(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10'))){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}}
    >
      <button
        className="card-open"
        aria-label={`Open ${item.title}`}
        onClick={event => {
          onSelect(item.id);
          item.onClick(event);
        }}
      >
        {item.icon ? (
          <div className="genre-art">
            <Icon name={item.icon} />
          </div>
        ) : (
          <div className="card-artwork" data-loading={photoPending}>
          <Cover
            src={item.cover}
            trackId={item.trackId}
            title={item.title}
            portrait={item.portrait}
          />
          {photoPending&&<span className="portrait-loading" data-photo={!!item.cover}><PortraitLoading/></span>}
          {item.playing&&<span className="card-playing" role="img" aria-label={item.isPlaying?'Now playing':'Current album, paused'}>{item.isPlaying?<LiveMeter/>:null}</span>}
          </div>
        )}
        <span className="card-caption">
          <strong title={item.title}>{item.title}</strong>
          <small title={item.subtitle}>{item.subtitle}</small>
        </span>
      </button>
      {item.menu && <Overflow items={item.menu} contextOnly />}
    </article>
  );
}
export function CollectionGrid({
  items,
  scrollRef,
  scale,
  initialOffset = 0,
  parallax = true,
  depth = false,
  selected,
  onSelect,
}: {
  items: CollectionItem[];
  scrollRef: RefObject<HTMLElement | null>;
  scale: number;
  initialOffset?: number;
  parallax?:boolean;
  depth?:boolean;
  selected?: string;
  onSelect: (id: string) => void;
}) {
  const element = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(500);
  const [margin, setMargin] = useState(0);
  useLayoutEffect(() => {
    if (!element.current) return;
    const node = element.current;
    let lastWidth=-1,sizeFrame=0;
    const readSize = () => {
      const next = node.clientWidth;
      if(next<=0)return; // A retained, hidden Library has no layout to measure.
      if(next!==lastWidth){lastWidth=next;setWidth(next);}
      if (scrollRef.current)
        {const nextMargin=node.getBoundingClientRect().top-scrollRef.current.getBoundingClientRect().top+scrollRef.current.scrollTop;
         setMargin(current=>Math.abs(current-nextMargin)>.25?nextMargin:current);}

    };
    const queueSize=()=>{if(!sizeFrame)sizeFrame=requestAnimationFrame(()=>{sizeFrame=0;readSize();});};
    const observer = new ResizeObserver(queueSize);
    window.addEventListener("resize",queueSize);
    window.visualViewport?.addEventListener("resize",queueSize);
    readSize();
    observer.observe(node);
    return () => { observer.disconnect();cancelAnimationFrame(sizeFrame);window.removeEventListener("resize",queueSize);window.visualViewport?.removeEventListener("resize",queueSize); };
  }, [scrollRef]);
  // Work in interface units, then apply the same geometry to both render paths.
  // Covers fill each row, growing up to 160 units before adding a column. Narrow
  // columns borrow from the gap first to keep artwork and text comfortably sized.
  const available = width / scale;
  const preferredGap = Math.min(42, Math.max(12, (available - 240) * 0.18));
  const columns = Math.max(1, Math.min(
    Math.ceil((available + preferredGap) / (160 + preferredGap)),
    Math.floor((available + 12) / (104 + 12)),
  ));
  const gap = (columns > 1
    ? Math.max(12, Math.min(preferredGap, (available - columns * 104) / (columns - 1)))
    : 0) * scale;
  const cardWidth = (width - gap * (columns - 1)) / columns;
  // Match the rendered 3.25rem caption plus .75rem row gutter. An inflated
  // estimate causes virtualized rows to jump as fresh measurements arrive.
  const height = cardWidth + 64 * scale;
  useGridReflow(element, columns, cardWidth);
  useArtworkParallax(element,scrollRef,parallax);
  useDepthArtwork(element,scrollRef,depth);
  const gridStyle: CSSProperties = {
    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
    columnGap: gap,
  };
  const rowKey = useCallback((index: number) => `${columns}:${index}`, [columns]);
  const virtual = useVirtualizer({
    useAnimationFrameWithResizeObserver:true,
    enabled: items.length > 120,
    initialOffset,
    count: Math.ceil(items.length / columns),
    getItemKey: rowKey,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => height,
    overscan: 3,
    scrollMargin: margin,
  });
  // Album rows have a known uniform height. Measurement compensation can
  // overwrite a native reverse fling with an older offset in WebKit.
  virtual.shouldAdjustScrollPositionOnItemSizeChange=()=>false;
  const virtualized = items.length > 120;
  // Stable keys preserve images, focus and hover state through every resize step.
  // Invalidate measurements rather than replacing all virtual row elements.
  useLayoutEffect(() => { if (virtualized) virtual.measure(); }, [virtual, virtualized, height, columns]);
  return (
    <div
      ref={element}
      className="collection-grid-container"
      data-parallax={parallax}
      style={virtualized ? { height: virtual.getTotalSize(), position: "relative" } : undefined}
    >
      {!virtualized ? <div className="collection-grid" style={gridStyle}>
        {items.map((item) => (
          <Card
            key={item.id}
            item={item}
            selected={selected}
            onSelect={onSelect}
          />
        ))}
      </div> : virtual.getVirtualItems().map((row) => (
        <div
          key={row.key}
          data-index={row.index}
          className="collection-grid virtual-grid"
          style={{
            ...gridStyle,
            top: row.start - margin,
            height,
          }}
        >
          {items
            .slice(row.index * columns, (row.index + 1) * columns)
            .map((item) => (
              <Card
                key={item.id}
                item={item}
                selected={selected}
                onSelect={onSelect}
              />
            ))}
        </div>
      ))}
    </div>
  );
}

const mixLabels: Record<QueueMix['kind'], string> = {
 beat_match: 'Beat match',
 outro_fade: 'Outro fade',
 crossfade: 'Crossfade',
 gapless: 'Natural handoff',
};

export function QueueTrackCopy({track,current=false,leadingLabel,mix}:{track:Track;current?:boolean;leadingLabel?:string;mix?:string}){
 return <span className="queue-track-copy">{leadingLabel&&<small className="queue-entry-label">{leadingLabel}</small>}<strong>{track.title}</strong>{(!leadingLabel||mix)&&<small>{mix&&<><span className="queue-mix-label">{mix}</span>{' · '}</>}{current?'Now playing · ':''}{track.artist}</small>}</span>;
}

export function QueueRows({tracks,entryIds,arranged=[],pinned=[],mixes,precedingEntry,current,scale,firstLabel,onSelect,onRemove,onMove}:{tracks:Track[];entryIds?:number[];arranged?:number[];pinned?:number[];mixes?:QueueMix[];precedingEntry?:number;current:number|null;scale:number;firstLabel?:string;onSelect:(index:number)=>void;onRemove:(index:number)=>void;onMove?:(from:number,to:number)=>void}) {
 const mixesByEntry = new Map(mixes?.map(mix=>[mix.incoming_entry,mix]));
 const ref=useRef<HTMLDivElement>(null),drag=useRef<{from:number;to:number;x:number;startY:number;y:number;pointer:number;moving:boolean;entry?:number}|null>(null),frame=useRef(0),suppressClick=useRef(false);
 const [target,setTarget]=useState<number|null>(null),[announcement,setAnnouncement]=useState('');
 const virtual=useVirtualizer({useAnimationFrameWithResizeObserver:true,count:tracks.length,getItemKey:index=>entryIds?.[index]??`${tracks[index].id}-${index}`,getScrollElement:()=>ref.current,estimateSize:()=>64*scale,overscan:7,enabled:tracks.length>100});
 const indices=tracks.length>100?virtual.getVirtualItems().map(v=>({index:v.index,top:v.start})):tracks.map((_,index)=>({index,top:0}));
 const finish=(commit:boolean)=>{const active=drag.current;drag.current=null;cancelAnimationFrame(frame.current);setTarget(null);if(active?.moving){suppressClick.current=true;if(commit&&active.from!==active.to&&(active.entry===undefined||entryIds?.[active.from]===active.entry)){onMove?.(active.from,active.to);haptic('drop',`queue:${active.entry??active.from}:${active.to}`);setAnnouncement(`${tracks[active.from].title} moved to position ${active.to+1}`);}}};
 const locate=(y:number,feedback=true)=>{const container=ref.current;if(!container||!drag.current)return;const nodes=[...container.querySelectorAll<HTMLElement>('[data-queue-index]')];let nearest=drag.current.to,distance=Infinity;for(const row of nodes){const rect=row.getBoundingClientRect(),d=Math.abs(y-(rect.top+rect.height/2));if(d<distance){distance=d;nearest=Number(row.dataset.queueIndex);}}if(feedback&&nearest!==drag.current.to)haptic('reorder',`queue:${nearest}`);drag.current.to=nearest;setTarget(nearest);};
 const finishRef=useRef(finish);finishRef.current=finish;
 useEffect(()=>{const escape=(e:KeyboardEvent)=>{if(e.key==='Escape'&&drag.current){e.preventDefault();e.stopImmediatePropagation();finishRef.current(false);}};window.addEventListener('keydown',escape,true);return()=>{window.removeEventListener('keydown',escape,true);cancelAnimationFrame(frame.current);};},[]);
 const autoscroll=()=>{const active=drag.current,container=ref.current;if(!active?.moving||!container)return;const box=container.getBoundingClientRect(),edge=40*scale;const delta=active.y<box.top+edge?-8*scale:active.y>box.bottom-edge?8*scale:0;if(delta){container.scrollTop+=delta;locate(active.y,false);}frame.current=requestAnimationFrame(autoscroll);};
 return <div className="queue-scroll" ref={ref}
 onPointerMove={e=>{const active=drag.current;if(!active||active.pointer!==e.pointerId)return;active.y=e.clientY;if(!active.moving&&Math.hypot(e.clientX-active.x,e.clientY-active.startY)>6*scale){active.moving=true;haptic('pickup',`queue:${active.entry??active.from}`);e.currentTarget.setPointerCapture(e.pointerId);frame.current=requestAnimationFrame(autoscroll);}if(active.moving){e.preventDefault();locate(e.clientY);}}}
 onPointerUp={e=>{if(drag.current?.pointer===e.pointerId)finish(true);}} onPointerCancel={()=>finish(false)} onLostPointerCapture={()=>finish(false)}><span className="sr-only" aria-live="polite">{announcement}</span><div style={tracks.length>100?{height:virtual.getTotalSize(),position:'relative'}:undefined}>{indices.map(({index,top})=>{const t=tracks[index],entry=entryIds?.[index],previous=index===0?precedingEntry:entryIds?.[index-1];const forecast=entry===undefined?undefined:mixesByEntry.get(entry),mix=forecast?.outgoing_entry===previous?forecast:undefined;return <div ref={tracks.length>100?virtual.measureElement:undefined} data-index={index} key={entryIds?.[index]??`${t.id}-${index}`} data-queue-index={index} onKeyDown={e=>{if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.stopPropagation();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}} onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} className={`queue-row ${index===current?'active':''} ${target===index?`drop-target ${drag.current&&index>drag.current.from?'drop-after':''}`:''}`} style={tracks.length>100?{position:'absolute',top,minHeight:60*scale,width:'100%'}:undefined}>
 <button className="queue-track" aria-label={`Play ${t.title} from queue`} aria-description={mix?`${mixLabels[mix.kind]}${mix.seconds>0?`, ${mix.seconds.toFixed(1)} seconds`:''}`:undefined} aria-current={index===current?'true':undefined} onDragStart={e=>e.preventDefault()}
 onPointerDown={e=>{if(!onMove||e.button!==0||e.pointerType==='touch')return;e.preventDefault();e.currentTarget.focus();suppressClick.current=false;drag.current={from:index,to:index,x:e.clientX,startY:e.clientY,y:e.clientY,pointer:e.pointerId,moving:false,entry:entryIds?.[index]};}}

 onKeyDown={e=>{if(onMove&&e.altKey&&(e.key==='ArrowUp'||e.key==='ArrowDown')){e.preventDefault();const to=Math.max(0,Math.min(tracks.length-1,index+(e.key==='ArrowUp'?-1:1)));if(to!==index){onMove(index,to);haptic('reorder',`queue:${to}`);setAnnouncement(`${t.title} moved to position ${to+1}`);requestAnimationFrame(()=>ref.current?.querySelector<HTMLElement>(`[data-queue-index="${to}"] .queue-track`)?.focus());}}}}
 onClick={e=>{if(suppressClick.current){e.preventDefault();suppressClick.current=false;return;}onSelect(index);}}><Cover songArtwork src={t.cover_art_path} trackId={t.id} title={t.album}/><QueueTrackCopy track={t} current={index===current} leadingLabel={index===0?firstLabel:undefined} mix={mix&&mixLabels[mix.kind]}/><span className="queue-duration">{entryIds&&arranged.includes(entryIds[index])&&<span title="Arranged by DJmode"><Icon name="DJmode"/></span>}{entryIds&&pinned.includes(entryIds[index])&&<span title="Position pinned by your move">⌖</span>}{duration(t.duration)}</span></button><span className="queue-context"><Overflow contextOnly items={[{label:"Play now",icon:"play",onSelect:()=>onSelect(index)},...(onMove&&index>0?[{label:"Play next",icon:"Play Next",onSelect:()=>onMove(index,0)}]:[]),{label:"Show in Finder",icon:"Open In Finder",disabled:!t.available,onSelect:()=>void call("reveal_track",{trackId:t.id}).catch(error=>setAnnouncement(`Could not show file: ${String(error)}`))},{label:"Remove from Up next",icon:"Song",onSelect:()=>onRemove(index)}]}/></span><button className="queue-remove" aria-label={`Remove ${t.title} from queue`} onClick={()=>onRemove(index)}>×</button></div>;})}</div></div>;
}
