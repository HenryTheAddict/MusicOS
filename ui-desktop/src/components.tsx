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
import { artwork, asset, native, localArtwork, libraryArtwork } from "./bridge";
import type { Track } from "./types";
import { SketchIcon } from "./SketchIcon";
import FavoriteButton from './FavoriteButton';
import {trackCredits} from './credits';
import {useArtworkParallax} from './motion';
import {useDepthArtwork} from './depthArtwork';
import {startSongDrag} from './songDrag';
import { useGridReflow } from "./motion";
import {usePortraitArtwork} from './portraitArtwork';
import {albumArtwork} from './albumArtwork';
export const Icon = SketchIcon;
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
      setExtracted(null);
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
  const candidate = canonical!==undefined ? canonical&&!failed.includes(canonical)?canonical:null :
    songArtwork && extracted && extracted.id===trackId && extracted.path && !failed.includes(extracted.path)
      ? extracted.path
      : src && !failed.includes(src)
      ? src
      : fallback && !failed.includes(fallback)
        ? fallback
        : extracted &&
            extracted.id === trackId &&
            extracted.path &&
            !failed.includes(extracted.path)
          ? extracted.path
          : null;
  useEffect(() => {
    if ((canonical!==undefined&&canonical!==null) || !native || (!songArtwork && src && !failed.includes(src)) || !trackId || !imageRef.current) return;
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
  }, [src, trackId, epoch, failed, songArtwork]);
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
  const framed=usePortraitArtwork(portrait?resolved:null,epoch);
  useLayoutEffect(()=>{const image=imageRef.current;onLoadingChange?.(!!resolved&&!(image?.complete&&image.naturalWidth>0));},[resolved,onLoadingChange]);
  return (
    <img
      ref={imageRef}
      data-artwork-path={resolved?(portrait?framed:resolved):undefined}
      data-artwork-version={epoch}
      data-original-artwork-path={resolved??undefined}
      data-full-src={resolved?artwork(resolved):undefined}
      className={`cover ${portrait ? "portrait" : ""} ${!resolved ? "placeholder" : ""}`}
      src={
        resolved
          ? artwork(portrait?framed:resolved) + (native && extracted?.path===resolved ? `?musicos-artwork=${epoch}` : "")
          : asset(
              `assets/icons/sketch/${portrait ? "noARTIST" : "noCOVER"}.svg`,
            )
      }
      alt={`${title} ${portrait ? "portrait" : "cover"}`}
      loading="lazy"
      onLoad={()=>onLoadingChange?.(false)}
      onError={() => {
        onLoadingChange?.(false);
        if (candidate && resolved) setFailed((old) => [...old, candidate]);
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
      onClick={onClick}
      disabled={disabled}
    >
      <Icon name={name} className="control-glyph" />
    </button>
  );
}
export interface MenuItem {
  label: string;
  onSelect: () => void;
  icon?: string;
  disabled?: boolean;
}
export function Menu({
  label,
  children,
  items,
  contextOnly = false,
}: {
  contextOnly?: boolean;
  label: string;
  children: ReactNode;
  items: MenuItem[];
}) {
  const [open,setOpen]=useState(false);
  const trigger=useRef<HTMLButtonElement>(null);
  useEffect(()=>{const button=trigger.current;const show=()=>setOpen(true);button?.addEventListener('musicos-open-menu',show);return()=>button?.removeEventListener('musicos-open-menu',show);},[]);
  return (
    <Dropdown.Root open={open} onOpenChange={setOpen}>
      <Dropdown.Trigger asChild>
        <button ref={trigger} className={`menu-trigger ${contextOnly?"context-only":""}`} tabIndex={contextOnly?-1:undefined} aria-label={label}>
          {children}
        </button>
      </Dropdown.Trigger>
      <Dropdown.Portal>
        <Dropdown.Content
          className="dropdown"
          sideOffset={6}
          collisionPadding={8}
          onCloseAutoFocus={contextOnly ? e=>{e.preventDefault();const row=trigger.current?.closest('.queue-row');if(row)row.querySelector<HTMLButtonElement>('.queue-track')?.focus();else trigger.current?.closest('.collection-card')?.querySelector<HTMLButtonElement>('.card-open')?.focus();} : undefined}
        >
          {items.map((item, i) => (
            <Dropdown.Item
              className="dropdown-item"
              key={`${item.label}-${i}`}
              onSelect={item.onSelect}
              disabled={item.disabled}
            >
              {item.icon && <Icon name={item.icon} />}
              <span>{item.label}</span>
            </Dropdown.Item>
          ))}
        </Dropdown.Content>
      </Dropdown.Portal>
    </Dropdown.Root>
  );
}
export function Overflow({ items, contextOnly=false }: { items: MenuItem[]; contextOnly?:boolean }) {
  return (
    <Menu label="More actions" items={items} contextOnly={contextOnly}>
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
  onEdit?: (track: Track) => void;
  onFavorite?: (track: Track) => void;
  playlistActions?: (track: Track) => MenuItem[];
  playingId?: number;
  isPlaying?: boolean;
  selected?: string;
  onSelect: (id: string) => void;
}) {
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
  const virtual = useVirtualizer({enabled:tracks.length>70,initialOffset,count:entries.length,getItemKey:i=>entries[i].track?.id??`disc-${entries[i].disc}`,getScrollElement:()=>scrollRef.current,estimateSize:i=>(entries[i].disc?38:44)*scale,gap:4*scale,overscan:8,scrollMargin:margin});
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
            data-index={virtualIndex}
            ref={tracks.length>70?virtual.measureElement:undefined}
            className={`track-row ${playingId === track.id ? "playing" : ""} ${selected === String(track.id) ? "selected" : ""}`}
            style={style}
            key={track.id}
            onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}
            onClick={() => onSelect(String(track.id))}
            onDoubleClick={() => track.available && onPlay(track)}
          >
            <span className="track-index" role="cell">
              <span>{multiDisc?track.track_number||index+1:index+1}</span>

            </span>
            <div className="track-title" role="cell" onPointerDown={e=>{if(track.available&&!(e.target as Element).closest("button"))startSongDrag(e,track);}}>
              <button
                className="row-play"
                disabled={!track.available}
                aria-label={`${playingId===track.id&&isPlaying?'Pause':'Play'} ${track.title}`}
                data-meter={playingId===track.id&&isPlaying}
                style={{'--level-1':levels?.[0]??0,'--level-2':levels?.[1]??0,'--level-3':levels?.[2]??0} as CSSProperties}
                onClick={(e) => {
                  e.stopPropagation();
                  if(playingId===track.id&&onToggle)onToggle();else onPlay(track);
                }}
              >
                <Icon name={playingId===track.id&&isPlaying?'rowLevels':'play'} />
              </button>
              <div className="track-label">
                <span title={track.title}>{track.title}</span>
                <div className="track-credits" title={track.artist} hidden={!!albumArtist&&trackCredits(track).length===1&&trackCredits(track)[0].toLocaleLowerCase()===albumArtist.trim().toLocaleLowerCase()}>{trackCredits(track).map((name,index)=><span key={name}>{index===1?' feat. ':index>1?', ':''}<button className="text-link muted" onClick={e=>{e.stopPropagation();onArtist(track,name);}}>{name}</button></span>)}{!track.available?' · Unavailable':''}</div>
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
            <FavoriteButton track={track} onFavorite={onFavorite}/>
            <Overflow
              items={[
                ...(playlistActions?.(track)??[]),
                {
                  label: "Play",
                  icon: "play",
                  onSelect: () => onPlay(track),
                  disabled: !track.available,
                },
                ...(onEdit ? [{label:"Edit metadata", icon:"File", onSelect:()=>onEdit(track)}] : []),
                {
                  label: "View Artist",
                  icon: "Artist",
                  onSelect: () => onArtist(track),
                },
              ]}
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
  onClick: () => void;
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
  const [imageLoading,setImageLoading]=useState(false);
  const photoPending=!!item.portrait&&(imageLoading||(!item.cover&&!!item.loading));
  return (
    <article
      className={`collection-card ${item.playing ? "currently-playing" : ""}`}
      data-collection-id={item.id}
      data-artist-name={item.portrait?item.title:undefined}
      onContextMenu={e=>{if(item.menu){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}}
      onKeyDown={e=>{if(item.menu&&(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10'))){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}}}
    >
      <button
        className="card-open"
        aria-label={`Open ${item.title}`}
        onClick={() => {
          onSelect(item.id);
          item.onClick();
        }}
      >
        {item.icon ? (
          <div className="genre-art">
            <Icon name={item.icon} />
          </div>
        ) : (
          <div className="card-artwork" data-loading={photoPending}>
          <Cover
            onLoadingChange={setImageLoading}
            src={item.cover}
            trackId={item.trackId}
            title={item.title}
            portrait={item.portrait}
          />
          {photoPending&&<span className="portrait-loading" data-photo={!!item.cover}>{item.cover&&<img src={artwork(item.cover)} alt="" draggable={false}/>}<PortraitLoading/></span>}
          {item.playing&&<span className="card-playing" role="img" aria-label={item.isPlaying?'Now playing':'Current album, paused'}><span className="equalizer" data-active={item.isPlaying}>{[0,1,2].map(i=><i key={i} style={{transform:`scaleY(${item.isPlaying?item.levels?.[i]??0:0})`}}/>)}</span></span>}
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
  const [settledWidth, setSettledWidth] = useState(500);
  const [margin, setMargin] = useState(0);
  useLayoutEffect(() => {
    if (!element.current) return;
    const node = element.current;
    let settle: ReturnType<typeof setTimeout>;
    let first = true;
    const observer = new ResizeObserver(([e]) => {
      const next = e.contentRect.width;
      setWidth(next);
      clearTimeout(settle);
      if (first) { setSettledWidth(next); first = false; }
      else settle = setTimeout(() => setSettledWidth(next), 140);
      if (scrollRef.current)
        setMargin(
          node.getBoundingClientRect().top -
            scrollRef.current.getBoundingClientRect().top +
            scrollRef.current.scrollTop,
        );
    });
    observer.observe(node);
    return () => { observer.disconnect(); clearTimeout(settle); };
  }, [scrollRef]);
  // Work in interface units, then apply the same geometry to both render paths.
  // Covers fill each row, growing up to 160 units before adding a column. Narrow
  // columns borrow from the gap first to keep artwork and text comfortably sized.
  const available = width / scale;
  const settledAvailable = settledWidth / scale;
  const settledGap = Math.min(42, Math.max(12, (settledAvailable - 240) * 0.18));
  const preferredGap = Math.min(42, Math.max(12, (available - 240) * 0.18));
  const columns = Math.max(1, Math.min(
    Math.ceil((settledAvailable + settledGap) / (160 + settledGap)),
    Math.floor((settledAvailable + 12) / (104 + 12)),
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
  useDepthArtwork(element,scrollRef,parallax&&depth);
  const gridStyle: CSSProperties = {
    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
    columnGap: gap,
  };
  const rowKey = useCallback((index: number) => `${columns}:${index}`, [columns]);
  const virtual = useVirtualizer({
    enabled: items.length > 120,
    initialOffset,
    count: Math.ceil(items.length / columns),
    getItemKey: rowKey,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => height,
    overscan: 3,
    scrollMargin: margin,
  });
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
          ref={virtual.measureElement}
          className="collection-grid virtual-grid"
          style={{
            ...gridStyle,
            transform: `translateY(${row.start - margin}px)`,
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

export function QueueRows({tracks,entryIds,arranged=[],pinned=[],current,scale,onSelect,onRemove,onMove}:{tracks:Track[];entryIds?:number[];arranged?:number[];pinned?:number[];current:number|null;scale:number;onSelect:(index:number)=>void;onRemove:(index:number)=>void;onMove?:(from:number,to:number)=>void}) {
 const ref=useRef<HTMLDivElement>(null),drag=useRef<{from:number;to:number;x:number;startY:number;y:number;pointer:number;moving:boolean;entry?:number}|null>(null),frame=useRef(0),suppressClick=useRef(false);
 const [target,setTarget]=useState<number|null>(null),[announcement,setAnnouncement]=useState('');
 const virtual=useVirtualizer({count:tracks.length,getItemKey:index=>entryIds?.[index]??`${tracks[index].id}-${index}`,getScrollElement:()=>ref.current,estimateSize:()=>64*scale,overscan:7,enabled:tracks.length>100});
 const indices=tracks.length>100?virtual.getVirtualItems().map(v=>({index:v.index,top:v.start})):tracks.map((_,index)=>({index,top:0}));
 const finish=(commit:boolean)=>{const active=drag.current;drag.current=null;cancelAnimationFrame(frame.current);setTarget(null);if(active?.moving){suppressClick.current=true;if(commit&&active.from!==active.to&&(active.entry===undefined||entryIds?.[active.from]===active.entry)){onMove?.(active.from,active.to);setAnnouncement(`${tracks[active.from].title} moved to position ${active.to+1}`);}}};
 const locate=(y:number)=>{const container=ref.current;if(!container||!drag.current)return;const nodes=[...container.querySelectorAll<HTMLElement>('[data-queue-index]')];let nearest=drag.current.to,distance=Infinity;for(const row of nodes){const rect=row.getBoundingClientRect(),d=Math.abs(y-(rect.top+rect.height/2));if(d<distance){distance=d;nearest=Number(row.dataset.queueIndex);}}drag.current.to=nearest;setTarget(nearest);};
 const finishRef=useRef(finish);finishRef.current=finish;
 useEffect(()=>{const escape=(e:KeyboardEvent)=>{if(e.key==='Escape'&&drag.current){e.preventDefault();e.stopImmediatePropagation();finishRef.current(false);}};window.addEventListener('keydown',escape,true);return()=>{window.removeEventListener('keydown',escape,true);cancelAnimationFrame(frame.current);};},[]);
 const autoscroll=()=>{const active=drag.current,container=ref.current;if(!active?.moving||!container)return;const box=container.getBoundingClientRect(),edge=40*scale;const delta=active.y<box.top+edge?-8*scale:active.y>box.bottom-edge?8*scale:0;if(delta){container.scrollTop+=delta;locate(active.y);}frame.current=requestAnimationFrame(autoscroll);};
 return <div className="queue-scroll" ref={ref}
 onPointerMove={e=>{const active=drag.current;if(!active||active.pointer!==e.pointerId)return;active.y=e.clientY;if(!active.moving&&Math.hypot(e.clientX-active.x,e.clientY-active.startY)>6*scale){active.moving=true;e.currentTarget.setPointerCapture(e.pointerId);frame.current=requestAnimationFrame(autoscroll);}if(active.moving){e.preventDefault();locate(e.clientY);}}}
 onPointerUp={e=>{if(drag.current?.pointer===e.pointerId)finish(true);}} onPointerCancel={()=>finish(false)} onLostPointerCapture={()=>finish(false)}><span className="sr-only" aria-live="polite">{announcement}</span><div style={tracks.length>100?{height:virtual.getTotalSize(),position:'relative'}:undefined}>{indices.map(({index,top})=>{const t=tracks[index];return <div ref={tracks.length>100?virtual.measureElement:undefined} data-index={index} key={entryIds?.[index]??`${t.id}-${index}`} data-queue-index={index} onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.dispatchEvent(new Event('musicos-open-menu'));}} className={`queue-row ${index===current?'active':''} ${target===index?`drop-target ${drag.current&&index>drag.current.from?'drop-after':''}`:''}`} style={tracks.length>100?{position:'absolute',top,minHeight:60*scale,width:'100%'}:undefined}>
 <button className="queue-track" aria-label={`Play ${t.title} from queue`} aria-current={index===current?'true':undefined} onDragStart={e=>e.preventDefault()}
 onPointerDown={e=>{if(!onMove||e.button!==0)return;e.preventDefault();e.currentTarget.focus();suppressClick.current=false;drag.current={from:index,to:index,x:e.clientX,startY:e.clientY,y:e.clientY,pointer:e.pointerId,moving:false,entry:entryIds?.[index]};}}

 onKeyDown={e=>{if(onMove&&e.altKey&&(e.key==='ArrowUp'||e.key==='ArrowDown')){e.preventDefault();const to=Math.max(0,Math.min(tracks.length-1,index+(e.key==='ArrowUp'?-1:1)));if(to!==index){onMove(index,to);setAnnouncement(`${t.title} moved to position ${to+1}`);requestAnimationFrame(()=>ref.current?.querySelector<HTMLElement>(`[data-queue-index="${to}"] .queue-track`)?.focus());}}}}
 onClick={e=>{if(suppressClick.current){e.preventDefault();suppressClick.current=false;return;}onSelect(index);}}><Cover songArtwork src={t.cover_art_path} trackId={t.id} title={t.album}/><span className="queue-track-copy"><strong>{t.title}</strong><small>{index===current?'Now playing · ':''}{t.artist}</small></span><span className="queue-duration">{entryIds&&arranged.includes(entryIds[index])&&<span title="Arranged by DJmode"><Icon name="DJmode"/></span>}{entryIds&&pinned.includes(entryIds[index])&&<span title="Position pinned by your move">⌖</span>}{duration(t.duration)}</span></button><span className="queue-context"><Overflow contextOnly items={[{label:"Play now",icon:"play",onSelect:()=>onSelect(index)},...(onMove&&index>0?[{label:"Play next",icon:"Song",onSelect:()=>onMove(index,0)}]:[]),{label:"Remove from Up next",icon:"Song",onSelect:()=>onRemove(index)}]}/></span><button className="queue-remove" aria-label={`Remove ${t.title} from queue`} onClick={()=>onRemove(index)}>×</button></div>;})}</div></div>;
}
