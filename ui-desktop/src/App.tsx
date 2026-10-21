import FavoriteButton from './FavoriteButton';
import OverflowLabel from './OverflowLabel';
import {playerTheme} from './playerTheme';
import {invalidateDisplay,type LibraryChanged} from './displayData';
import {KeepView} from './ViewScope';
import {mergePlaybackUpdate} from './playbackPresentation';
import {captureAlbumCover,captureAlbumReturn,captureArtistCover,captureArtistReturn,captureLibraryToolbar} from './albumTransitions';
import MiniPlayer from './MiniPlayer';
import usePanelPresence from './usePanelPresence';
import useAnalyzedTempos from './useAnalyzedTempos';
import {dragWindow} from './windowDrag';
import {TrackSelection} from './TrackSelection';
import {appearance,themePalette,readableInk} from './appearance';
import {albumArtists,trackCredits} from './credits';
import ThemedSwitch from './ThemedSwitch';
import LibraryFilter,{defaultFilters,filterAlbums} from './LibraryFilter';
import {registerAlbumArtwork} from './albumArtwork';
import QuickAudio from "./QuickAudio";
import AudioPanel from "./AudioPanel";
import ArtworkViewer,{openArtwork} from "./ArtworkViewer";
import AlbumArtwork from './AlbumCoverMotion';
import AlbumArtists from "./AlbumArtists";
import CollectionCreate from "./CollectionDesign";
import PlaylistPicker from "./PlaylistPicker";
const SparkCreate = lazy(()=>import("./SparkCreate"));
import VolumeControl from "./VolumeControl";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  call,
  artwork,
  subscribe,
  native,
  localArtwork,
  errorText,
  emptyLibrary,
  emptyPlayback,
  emptyTask,
  defaultPreferences,
} from "./bridge";
import {
  navigationReducer,
  restoreNavigation,
  themeSource,
} from "./navigation";
import {
  CollectionGrid,
  Cover,
  Empty,
  Icon,
  IconButton,
  Menu,
  Overflow,
  TrackRows,
  collectionDuration,
  duration,
  QueueRows,
  type CollectionItem,
} from "./components";
import {PlaylistDetail,PlaylistArtwork,AudioSparkDetail} from './PlaylistLibrary';
const CollectionManager = lazy(()=>import("./CollectionManager"));
const AudioAnalysis = lazy(()=>import("./AudioAnalysis"));
const Settings = lazy(()=>import("./Settings"));
import ArtistOverview from "./ArtistOverview";
import { resolveArtworkTheme, clearThemeCache } from "./artworkTheme";
import { useNativeChrome } from "./nativeChrome";
const Devices = lazy(()=>import("./Devices"));
const LibraryTools = lazy(()=>import("./LibraryTools"));
import { usePageEntrance } from "./motion";
import { useSmoothCorners } from "./smoothCorners";
import PlaybackProgress from "./PlaybackProgress";
import AudioQuality from "./AudioQuality";
import AlbumDescription from "./AlbumDescription";
const SuperCommand = lazy(()=>import("./SuperCommand"));
import type {
  SavedCollection,
  Album,
  Artist,
  ArtistInfo,
  Library,
  Palette,
  Playback,
  PlaybackAction,
  Preferences,
  Route,
  Scope,
  Task,
  Track,
} from "./types";
const scopeIcons: Record<Scope, string> = {
  albums: "Album",
  artists: "Artist",
  genres: "Genre",
  songs: "Song",
  playlists:'Playlist',
  sparklists:'Sparklist',
};

const profileCache = new Map<string, Promise<ArtistInfo>>();
function contrastInk(hex:string){
 const rgb=hex.replace('#','').match(/.{2}/g)?.map(v=>parseInt(v,16)/255);
 if(!rgb||rgb.length!==3)return '#fff';
 const linear=rgb.map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);
 return linear[0]*.2126+linear[1]*.7152+linear[2]*.0722>.179?'#111820':'#fff';
}
export default function App() {
  const [settingsLibraryView,setSettingsLibraryView]=useState<"sources"|"review"|"playlists"|"sparklists">("sources");
  const [metadataIds,setMetadataIds]=useState<number[]>([]);
  function openManagement(view:"sources"|"review"|"playlists"|"sparklists",ids:number[]=[]){captureScroll();setSettingsLibraryView(view);setMetadataIds(ids);sessionStorage.setItem("musicos-settings-tab","Library");window.dispatchEvent(new CustomEvent("musicos-settings-tab",{detail:"Library"}));setSection("Settings");}
  const [collections,setCollections]=useState<SavedCollection[]>([]);
  const [rowSelection,setRowSelection]=useState<Set<number>>(new Set());
  const rowAnchor=useRef(0);
  const [playlistTrack,setPlaylistTrack]=useState<(Track&{selectionIds?:number[]})|null>(null);
  const [creatingCollection,setCreatingCollection]=useState<"manual"|"spark"|null>(null);
  const [collectionError,setCollectionError]=useState("");
  useEffect(()=>{const drag=(event:Event)=>{const value=(event as CustomEvent<import('./songDrag').SongDrag>).detail;document.querySelectorAll('[data-song-drop]').forEach(el=>el.removeAttribute('data-song-drop'));const target=document.elementFromPoint(value.x,value.y)?.closest<HTMLElement>('[data-manual-collection]');if(!target||value.cancelled)return;if(!value.drop){target.dataset.songDrop='true';return;}void call('collections',{request:{action:'add',id:target.dataset.manualCollection,track_ids:value.ids??[value.id]}}).then(setCollections).catch(e=>setCollectionError(errorText(e)));};window.addEventListener('musicos-song-drag',drag);return()=>window.removeEventListener('musicos-song-drag',drag);},[]);
  const favoritePending=useRef(new Map<number,Promise<void>>());

  const [library, setRawLibrary] = useState<Library>(emptyLibrary);
  const setLibrary:typeof setRawLibrary=next=>setRawLibrary(old=>{const value=typeof next==='function'?next(old):next;registerAlbumArtwork(value);return value;});
  const tracksById=useMemo(()=>new Map(library.tracks.map(t=>[t.id,t])),[library.tracks]);
  const librarySummary=useMemo(()=>({favorites:library.tracks.filter(t=>t.favorited).length,duration:library.tracks.reduce((sum,t)=>sum+t.duration,0)}),[library.tracks]);
  const collectionSummaries=useMemo(()=>new Map(collections.map(item=>{let duration=0,available=0;for(const id of item.track_ids){const track=tracksById.get(id);if(track){duration+=track.duration;available++;}}return [item.id,{duration,available}];})),[collections,tracksById]);
  const [preferences, setPreferences] = useState(defaultPreferences);
  useSmoothCorners();
  const [miniMode,setMiniMode]=useState<'normal'|'tall'|'queue'>('normal');
  const miniChanging=useRef(false);
  const [miniPinned,setMiniPinned]=useState(true),[windowPinned,setWindowPinned]=useState(false);
  useNativeChrome(miniMode==='normal');
  const [rawPlayer, setPlayer] = useState<Playback>(emptyPlayback);
  const presentedQueue=useMemo(()=>rawPlayer.queue.map(t=>tracksById.get(t.id)??t),[rawPlayer.queue,tracksById]);
  const player=useMemo(()=>({...rawPlayer,track:rawPlayer.track?(tracksById.get(rawPlayer.track.id)??rawPlayer.track):null,queue:presentedQueue}),[rawPlayer,tracksById,presentedQueue]);
  const [task, setTask] = useState<Task>(emptyTask);
  const [loading, setLoading] = useState(true);
  const tempoIdentity=useMemo(()=>library.tracks.map(track=>`${track.id}:${track.path}`).join('\n'),[library.tracks]);
  const tempos=useAnalyzedTempos(!loading,tempoIdentity,player.track?.id);
  const [error, setError] = useState("");
  const [dismissedPlaybackError, setDismissedPlaybackError] = useState<
    string | null
  >(null);
  const visiblePlaybackError =
    player.error !== dismissedPlaybackError ? player.error : null;
  useEffect(() => {
    if (!player.error) setDismissedPlaybackError(null);
  }, [player.error]);
  const [saveError, setSaveError] = useState("");
  const [section, setSection] = useState<"Library" | "Devices" | "Settings">(
    "Library",
  );
  const [fileDrop,setFileDrop]=useState(false);
  const [pendingImport, setPendingImport] = useState<{
    paths: string[];
    folders: boolean;
  } | null>(null);
  const importDialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (pendingImport) importDialog.current?.showModal();
  }, [pendingImport]);
  const [queueOpen, setQueueOpen] = useState(false);
  const queuePresent=usePanelPresence(queueOpen);
  useEffect(()=>{const open=()=>{setQueueOpen(false);setSection('Settings');sessionStorage.setItem('musicos-settings-tab','Audio');sessionStorage.setItem('musicos-audio-page','quality');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));window.dispatchEvent(new CustomEvent('musicos-audio-page',{detail:'quality'}));};window.addEventListener('musicos-open-audio-settings',open);return()=>window.removeEventListener('musicos-open-audio-settings',open);},[]);
  const [commandOpen,setCommandOpen]=useState(false);
  const [artistLoading,setArtistLoading]=useState<Set<string>>(()=>new Set());
  const artistCompletions=useRef(new Map<string,number>());
  const [artistInfo, setArtistInfo] = useState<ArtistInfo | null>(null);
  const [resolvedThemeCover, setResolvedThemeCover] = useState<string | null>(null);
  useEffect(()=>{if(loading)return;const warm=()=>{void import('./Settings');void import('./CollectionManager');if(native)void call('library_ready').catch(()=>{});};const id=window.setTimeout(warm,1200);return()=>clearTimeout(id);},[loading]);
  const [themeEpoch, setThemeEpoch] = useState(0);
  useEffect(() => { const clear = () => { clearThemeCache(); setThemeEpoch(n=>n+1); }; window.addEventListener("musicos-artwork-invalidated", clear); return () => window.removeEventListener("musicos-artwork-invalidated", clear); }, []);
  const [artPalette,setArtPalette]=useState<Palette|null>(null);
  const [systemDark,setSystemDark]=useState(()=>matchMedia('(prefers-color-scheme: dark)').matches);
  useEffect(()=>{const media=matchMedia('(prefers-color-scheme: dark)');const change=()=>setSystemDark(media.matches);media.addEventListener('change',change);return()=>media.removeEventListener('change',change);},[]);
  const look=appearance(preferences);
  useEffect(()=>{document.documentElement.dataset.themeMode=look.mode==='system'?(systemDark?'dark':'light'):look.mode;},[look.mode,systemDark]);
  const palette=useMemo(()=>themePalette(look,artPalette,systemDark),[look.mode,look.source,look.accent,look.saturation,look.tint,artPalette,systemDark]);
  useLayoutEffect(()=>{document.documentElement.dataset.playerLayout=miniMode;return()=>{delete document.documentElement.dataset.playerLayout;};},[miniMode]);
  const launchThemeSettled=useRef(false);
  const [themeResolved,setThemeResolved]=useState(false);
  useLayoutEffect(()=>{document.documentElement.dataset.themePhase="launch";return()=>{delete document.documentElement.dataset.themePhase;delete document.documentElement.dataset.themeMotion;};},[]);
  useEffect(()=>{
    if(launchThemeSettled.current||!themeResolved)return;
    const timer=setTimeout(()=>{launchThemeSettled.current=true;document.documentElement.dataset.themePhase="ready";},1600);
    return()=>clearTimeout(timer);
  },[themeResolved]);
  const appliedTheme=useRef<{appearance:string;resolved:boolean;palette:Palette|null}|null>(null);
  const appearanceKey=JSON.stringify([look,look.mode==='system'?systemDark:null]);
  useLayoutEffect(()=>{
    const previous=appliedTheme.current;
    if(previous?.appearance!==appearanceKey||look.source!=='artwork'){
      delete document.documentElement.dataset.themeMotion;
      if(previous?.resolved&&previous.appearance!==appearanceKey){launchThemeSettled.current=true;document.documentElement.dataset.themePhase='ready';}
    }
    else if(previous.resolved&&themeResolved&&previous.palette!==palette)document.documentElement.dataset.themeMotion='artwork';
    const tokens={'--page-color':palette?.background??(section==='Library'?'#00aeff':'#c9efff'),'--shell-color':palette?.blob??'#00baff','--accent':palette?.accent??'#0064df','--surface-color':palette?.background??'#cbf1ff','--blob-color':palette?.blob??'#54b9ff','--ink':palette?.text??'#111820','--accent-ink':look.source==='artwork'?(look.mode==='dark'||(look.mode==='system'&&systemDark)?'#000000':'#ffffff'):readableInk(palette?.accent??'#0064df')};
    document.documentElement.style.setProperty('--playback-tone',tokens['--ink']);
    for(const [name,value] of Object.entries(tokens))document.documentElement.style.setProperty(name,value);
    appliedTheme.current={appearance:appearanceKey,resolved:themeResolved,palette};
  },[palette,section,themeResolved,appearanceKey]);
  const [navigation, dispatch] = useReducer(navigationReducer, undefined, () =>
    restoreNavigation(sessionStorage.getItem("musicos-navigation-v1")),
  );
  const route = navigation.current;
  const liveView=useRef({section,miniMode}),scrollRestoring=useRef(false);
  if(liveView.current.section!==section||liveView.current.miniMode!==miniMode)scrollRestoring.current=true;
  liveView.current={section,miniMode};
  const scrollRef = useRef<HTMLElement>(null);
  const settingsScroll=useRef(new Map<string,number>()),settingsKey=useRef(sessionStorage.getItem('musicos-settings-tab')??'Appearance');
  useLayoutEffect(()=>{
    let frame=0,settled=0,generation=0;
    const save=()=>{if(!scrollRestoring.current&&liveView.current.section==='Settings'&&liveView.current.miniMode==='normal'&&scrollRef.current)settingsScroll.current.set(settingsKey.current,scrollRef.current.scrollTop);};
    const restore=()=>{const current=++generation,position=settingsScroll.current.get(settingsKey.current)??0;scrollRestoring.current=true;cancelAnimationFrame(frame);cancelAnimationFrame(settled);frame=requestAnimationFrame(()=>{if(current!==generation)return;if(scrollRef.current)scrollRef.current.scrollTop=position;settled=requestAnimationFrame(()=>{if(current===generation)scrollRestoring.current=false;});});};
    const tab=(event:Event)=>{save();settingsKey.current=String((event as CustomEvent).detail);restore();};
    const node=scrollRef.current;node?.addEventListener('scroll',save,{passive:true});window.addEventListener('musicos-settings-selected',tab);
    if(section==='Settings'&&miniMode==='normal')restore();else scrollRestoring.current=false;
    return()=>{generation++;cancelAnimationFrame(frame);cancelAnimationFrame(settled);node?.removeEventListener('scroll',save);window.removeEventListener('musicos-settings-selected',tab);};
  },[section,miniMode]);

  usePageEntrance(scrollRef, `${section}:${route.view}:${route.id ?? ""}`, !loading&&miniMode==='normal');
  const savePending = useRef<Preferences|null>(null);
  const savingPreferences=useRef(false);
  const scale = preferences.interface_percent / 100;
  const mounted = useRef(true);
  const refreshWork=useRef<Promise<void>|null>(null),refreshAgain=useRef(false);
  const refresh = useCallback(() => {
    if(refreshWork.current){refreshAgain.current=true;return refreshWork.current;}
    const work=(async()=>{do{refreshAgain.current=false;try{
      const next=await call("refresh_library");
      setLibrary(next);invalidateDisplay("album_metadata");
      setCollections(await call("collections",{}));setCollectionError('');
      dispatch({type:"reconcile",albums:next.albums.map(a=>a.id),artists:next.artists.map(a=>a.id),genres:next.genres.map(g=>g.id)});
    }catch(e){setError(errorText(e));}}while(refreshAgain.current);})();
    refreshWork.current=work;void work.finally(()=>{refreshWork.current=null;});return work;
  }, []);
  useEffect(() => {
    mounted.current = true;
    let cleanups: (() => void)[] = [];
    let gone = false;
    const lightweight=call("bootstrap_state").then(data=>{
      if(gone)return;setPreferences(data.preferences);setPlayer(data.playback);setTask(data.task);
      if(data.preference_error)setSaveError(data.preference_error);
    });
    const libraryLoad=call("refresh_library").then(data=>{if(!gone)setLibrary(data);});
    void Promise.all([lightweight,libraryLoad]).then(()=>{
      if(!gone)void call("collections",{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));
    }).catch(e=>{if(!gone)setError(errorText(e));}).finally(()=>{if(!gone)setLoading(false);});
    async function events() {
      const listeners = await Promise.all([
        subscribe<Partial<Playback>>("playback", (update) =>
          setPlayer((old) => mergePlaybackUpdate(old,update)),
        ),
        subscribe<Task>("task", setTask),
        subscribe<LibraryChanged | null>("library-changed", (change) => {if(change?.reason!=="analysis"&&change?.reason!=="favorites")invalidateDisplay();window.dispatchEvent(new CustomEvent("musicos-library-changed",{detail:change}));void refresh();}),
        subscribe("audio-rescanned",()=>window.dispatchEvent(new Event("musicos-audio-rescanned"))),
        subscribe("artist-profile", (payload:any)=>{artistCompletions.current.set(payload.name,(artistCompletions.current.get(payload.name)??0)+1);setArtistLoading(old=>{const next=new Set(old);next.delete(payload.name);return next;});profileCache.set(payload.name,Promise.resolve(payload.info));setLibrary(old=>({...old,artists:old.artists.map(a=>a.name===payload.name?{...a,portrait:payload.info.portrait}:a)}));if(artistNameRef.current===payload.name)setArtistInfo(payload.info);}),
        subscribe<{track_id:number;path:string}>("song-artwork-found",update=>setLibrary(old=>({...old,tracks:old.tracks.map(t=>t.id===update.track_id?{...t,cover_art_path:update.path}:t)}))),
        subscribe<{ track_id: number; path: string }>(
          "artwork-found",
          (update) =>
            setLibrary((old) => ({
              ...old,
              tracks: old.tracks.map((t) =>
                t.id === update.track_id
                  ? { ...t, cover_art_path: update.path }
                  : t,
              ),
              albums: old.albums.map((a) =>
                a.track_ids.includes(update.track_id)
                  ? { ...a, cover: update.path }
                  : a,
              ),
            })),
        ),
      ]);
      if (gone) listeners.forEach((f) => f());
      else cleanups.push(...listeners);
      if (native) {
        const unlisten = await getCurrentWebviewWindow().onDragDropEvent(
          (e) => {
            if(e.payload.type==="enter"&&e.payload.paths.length>0)setFileDrop(true);
            if(e.payload.type==="leave"||e.payload.type==="drop")setFileDrop(false);
            if (e.payload.type === "drop") {
              const paths = e.payload.paths;
              if (!paths.length) return;
              void call("inspect_import", { paths })
                .then(() => call("library_task", {request:{kind:"copy",paths}}))
                .catch((e) => setError(errorText(e)));
            }
          },
        );
        if (gone) unlisten();
        else cleanups.push(unlisten);
      }
    }
    void events().catch((e) => setError(errorText(e)));
    return () => {
      gone = true;
      mounted.current = false;
      cleanups.forEach((f) => f());
    };
  }, [refresh]);
  useEffect(() => {
    try {
      sessionStorage.setItem(
        "musicos-navigation-v1",
        JSON.stringify(navigation),
      );
    } catch {}
  }, [navigation]);
  useLayoutEffect(() => {
    document.documentElement.style.fontSize = `${16 * scale}px`;
    return () => {
      document.documentElement.style.fontSize = "";
    };
  }, [scale]);
  useLayoutEffect(() => {
    if (section === "Library" && !loading && scrollRef.current)
      scrollRef.current.scrollTop = route.scroll;
  }, [route.view, route.id, section, loading]);

  const album = library.albums.find((a) => a.id === route.id||a.legacy_ids?.includes(route.id??""));
  const artist = library.artists.find((a) => a.id === route.id||a.legacy_ids?.includes(route.id??""));
  const artistCoverAlbums = useMemo(() => {
    const covers = new Map<string, Album>();
    const artistsByAlbum = new Map<string, string[]>();
    for (const artist of library.artists) {
      for (const id of artist.album_ids) {
        const artists = artistsByAlbum.get(id) ?? [];
        artists.push(artist.id);
        artistsByAlbum.set(id, artists);
      }
    }
    for (const album of library.albums) {
      for (const id of artistsByAlbum.get(album.id) ?? []) {
        if (!covers.has(id) || (!covers.get(id)?.cover && album.cover)) covers.set(id, album);
      }
    }
    return covers;
  }, [library.albums,library.artists]);
  const firstArtistAlbum = artist
    ? artistCoverAlbums.get(artist.id)
    : undefined;
  const artistNameRef=useRef(artist?.name);artistNameRef.current=artist?.name;
  const enrichArtist=useCallback(async (name:string)=>{
    const started=artistCompletions.current.get(name)??0;
    let request=profileCache.get(name);
    if(!request){request=call("artist_info",{name,refresh:false});profileCache.set(name,request);request.catch(()=>profileCache.delete(name));}
    setArtistLoading(old=>new Set(old).add(name));
    let pending=false;
    try {const info=await request;pending=info.lookup_status==='loading'&&started===(artistCompletions.current.get(name)??0);
    if(info.portrait&&started===(artistCompletions.current.get(name)??0))setLibrary(old=>({...old,artists:old.artists.map(a=>a.name===name&&a.portrait!==info.portrait?{...a,portrait:info.portrait}:a)}));
    return info;
    }finally{if(!pending)setArtistLoading(old=>{const next=new Set(old);next.delete(name);return next;});}
  },[]);
  useEffect(()=>{
    let gone=false;setArtistInfo(null);
    const names=route.view==='artist'&&artist?[artist.name]:route.view==='album'&&album?library.artists.filter(a=>a.album_ids.includes(album.id)).map(a=>a.name):[];
    void (async()=>{for(const name of names){if(gone)return;try{if(route.view==='artist')profileCache.delete(name);const info=route.view==='artist'?await call('artist_info',{name,refresh:true}):await enrichArtist(name);if(!gone&&route.view==='artist')setArtistInfo(info);}catch(e){if(!gone&&route.view==='artist')setArtistInfo(null);}}})();
    return()=>{gone=true;};
  },[route.view,route.id,artist?.name,album?.id,enrichArtist]);
  useEffect(()=>{
    if(route.view!=='artists'||section!=='Library')return;
    let gone=false;
    const pending:string[]=[];let running=false;
    const pump=async()=>{if(running)return;running=true;while(pending.length&&!gone){try{await enrichArtist(pending.shift()!);}catch{}}running=false;};
    const observer=new IntersectionObserver(entries=>{for(const entry of entries){if(!entry.isIntersecting)continue;observer.unobserve(entry.target);const name=entry.target.getAttribute('data-artist-name');if(name&&!gone&&!pending.includes(name)){pending.push(name);setArtistLoading(old=>new Set(old).add(name));}}void pump();},{root:scrollRef.current,rootMargin:'100px'});
    const observed=new WeakSet<Element>();
    const scan=()=>document.querySelectorAll('[data-artist-name]').forEach(el=>{if(!observed.has(el)){observed.add(el);observer.observe(el);}});
    scan();const mutations=new MutationObserver(scan);if(scrollRef.current)mutations.observe(scrollRef.current,{childList:true,subtree:true});
    return()=>{gone=true;observer.disconnect();mutations.disconnect();setArtistLoading(old=>{const next=new Set(old);pending.forEach(name=>next.delete(name));return next;});};
  },[route.view,section,enrichArtist]);
  function openCollection(item:SavedCollection){captureScroll();setSection('Library');dispatch({type:'open',route:{view:'playlist',id:item.id,search:'',scroll:0}});}
  function favorite(track:Track){
    setLibrary(old=>({...old,tracks:old.tracks.map(t=>t.id===track.id?{...t,favorited:!t.favorited}:t)}));
    const previous=favoritePending.current.get(track.id)??Promise.resolve();
    const work=previous.then(async()=>{try{const value=await call('toggle_favorite',{trackId:track.id});if(favoritePending.current.get(track.id)===work)setLibrary(old=>({...old,tracks:old.tracks.map(t=>t.id===track.id?{...t,favorited:value}:t)}));}catch(e){setLibrary(old=>({...old,tracks:old.tracks.map(t=>t.id===track.id?{...t,favorited:!t.favorited}:t)}));setError(errorText(e));}}).finally(()=>{if(favoritePending.current.get(track.id)===work){favoritePending.current.delete(track.id);void call('collections',{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));}});
    favoritePending.current.set(track.id,work);
  }
  const playingAlbum=library.albums.find(a=>a.track_ids.includes(player.track?.id??-1));
  const playingCover = playingAlbum ? playingAlbum.cover :
    library.tracks.find(t=>t.id===player.track?.id)?.cover_art_path ??
    player.track?.cover_art_path ??
    library.albums.find((a) => a.track_ids.includes(player.track?.id ?? -1))
      ?.cover;
  const [playingArtPalette,setPlayingArtPalette]=useState<Palette|null>(null);
  const playingThemeRequest=JSON.stringify([playingCover??null,playingAlbum?.track_ids[0]??player.track?.id]);
  useEffect(()=>{
    let gone=false;
    if(loading||look.source!=='artwork'){setPlayingArtPalette(null);return;}
    const [cover,id]=JSON.parse(playingThemeRequest) as [string|null,number|undefined];
    void resolveArtworkTheme([cover],id).then(result=>{if(!gone)setPlayingArtPalette(result?.palette??null);});
    return()=>{gone=true;};
  },[loading,look.source,playingThemeRequest,themeEpoch]);
  const playingPalette=useMemo(()=>themePalette(look,playingArtPalette,systemDark),[look.mode,look.source,look.accent,look.saturation,look.tint,playingArtPalette,systemDark]);
  const playingStyle=useMemo(()=>playerTheme(playingPalette),[playingPalette]);
  const source = themeSource(
    section === "Library"
      ? route.view === "catalog" && route.catalog?.kind === "album"
        ? "album"
        : route.view
      : "albums",
    route.view === "catalog" ? route.catalog?.cover : album?.cover,
    artistInfo?.portrait ?? artist?.portrait,
    firstArtistAlbum?.cover,
    playingCover,
  );
  const artistThemeTracks=useMemo(()=>library.albums.filter(a=>artist?.album_ids.includes(a.id)).map(a=>a.track_ids[0]).filter((id):id is number=>id!==undefined),[library.albums,artist?.id]);
  const themeTrackId = section === "Library" && route.view === "album"
    ? album?.track_ids[0]
    : section === "Library" && route.view === "artist" ? artistThemeTracks : playingAlbum?.track_ids[0]??player.track?.id;
  const themeRequest=JSON.stringify([[source,section==='Library'&&route.view==='artist'?firstArtistAlbum?.cover:null],themeTrackId]);
  useEffect(() => {
    let gone=false;
    async function resolve() {
      if(loading)return;
      const [paths,tracks]=JSON.parse(themeRequest) as [(string|null)[],number|number[]|undefined];
      const result=look.source==='artwork'?await resolveArtworkTheme(paths,tracks??undefined):null;
      if(!gone){setResolvedThemeCover(result?.path??null);setArtPalette(result?.palette??null);setThemeResolved(true);}
    }
    void resolve();return ()=>{gone=true;};
  }, [loading,look.source,themeRequest,themeEpoch]);
  const run = useCallback(async (action: PlaybackAction) => {
    try {
      setPlayer(await call("playback", { action }));
    } catch (e) {
      setError(errorText(e));
    }
  }, []);
  useEffect(() => {
    const listener = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      const target = e.target as HTMLElement;
      if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){
        e.preventDefault();setCommandOpen(value=>!value);return;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        chooseSection("Settings");
        return;
      }
      if (e.altKey && e.key === "ArrowLeft" && !target.closest("[role=menu]")) {
        e.preventDefault();
        goBack();
        return;
      }
      if (
        e.key === "Escape" &&
        queueOpen &&
        !target.closest("dialog,[role=menu]")
      ) {
        setQueueOpen(false);
        return;
      }
      if (
        target.closest(
          "input,textarea,select,button,a,summary,dialog,[contenteditable=true],[role=menu],[role=button]",
        )
      )
        return;
      if (e.code === "Space" && player.track) {
        e.preventDefault();
        void run({ type: "toggle" });
      }
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [run, section, queueOpen, player.track?.id]);
  function changePreferences(next: Preferences) {
    // A deliberate theme choice must not continue the launch interpolation:
    // intermediate light/dark ink and surfaces can be unreadable together.
    if(JSON.stringify(appearance(next))!==JSON.stringify(look)){
      launchThemeSettled.current=true;
      document.documentElement.dataset.themePhase='ready';
    }
    setPreferences(next);
    setSaveError("");
    savePending.current=next;
    if(savingPreferences.current)return;
    savingPreferences.current=true;
    void (async()=>{
      try {while(savePending.current){const latest=savePending.current;savePending.current=null;try {await call("save_preferences",{preferences:latest});if(mounted.current)setSaveError("");}catch(e){if(mounted.current)setSaveError(`Could not save settings. ${errorText(e)}`);}}}
      finally {savingPreferences.current=false;}
    })();
  }
  function captureScroll() {
    if(section==='Settings'&&scrollRef.current)settingsScroll.current.set(settingsKey.current,scrollRef.current.scrollTop);
    if (section === "Library" && scrollRef.current)
      dispatch({
        type: "patch",
        patch: { scroll: scrollRef.current.scrollTop },
      });
  }
  function goBack() {
    if(section==='Library'&&route.view==='album')captureAlbumReturn(scrollRef.current);
    else if(section==='Library'&&route.view==='artist')captureArtistReturn(scrollRef.current);
    captureScroll();
    dispatch({ type: "back" });
  }
  function chooseSection(next: "Library" | "Devices" | "Settings") {
    captureScroll();
    setSection(next);
  }
  function openRoute(next: Route) {
    if(section==='Library'&&['album','artist'].includes(next.view))captureLibraryToolbar(scrollRef.current);
    captureScroll();
    dispatch({ type: "open", route: next });
  }
  function goScope(view: Route["view"]) {
    captureScroll();
    dispatch({ type: "scope", view });
  }
  function openAlbum(a: Album,from?:HTMLElement) {
    captureAlbumCover(a.id,from);
    openRoute({ view: "album", id: a.id, search: "", scroll: 0 });
  }
  function openArtist(a: Artist,from?:HTMLElement) {
    captureArtistCover(a.id,from);
    openRoute({ view: "artist", id: a.id, search: "", scroll: 0 });
  }
  function artistFor(track: Track, name?:string) {
    const a = library.artists.find((a) => a.track_ids.includes(track.id)&&(!name||a.name===name||a.credited_names?.includes(name)));
    if (a) openArtist(a);
  }
  function albumFor(track: Track) {
    const a = library.albums.find((a) => a.track_ids.includes(track.id));
    if (a) openAlbum(a);
  }
  const availableIds = useMemo(
    () => new Set(library.tracks.filter((t) => t.available).map((t) => t.id)),
    [library.tracks],
  );
  function playIds(ids: number[], shuffle = false, startId?: number) {
    const playable = ids.filter((id) => availableIds.has(id));
    const index = startId === undefined ? 0 : playable.indexOf(startId);
    void run({ type: "play", ids: playable, start_index: index, shuffle });
  }
  async function chooseFolder(copy = false) {
    try {
      if (!native)
        throw Error("Open the MusicOS desktop app to add your music.");
      const result = await open({
        directory: true,
        multiple: true,
        title: copy ? "Copy music into MusicOS" : "Add a music folder",
      });
      if (!result) return;
      await call("library_task", {
        request: {
          kind: copy ? "copy" : "reference",
          paths: typeof result === "string" ? [result] : result,
        },
      });
    } catch (e) {
      setError(errorText(e));
    }
  }
  function startTask(kind: string, root?: string) {
    void call("library_task", { request: { kind, root } }).catch((e) =>
      setError(errorText(e)),
    );
  }
  const needle = route.search.toLocaleLowerCase().trim();
  const match = (...s: string[]) =>
    !needle || s.some((v) => v.toLocaleLowerCase().includes(needle));
  const tracks = useMemo(() => {
    let records = library.tracks;
    if (route.view === "album")
      records = (album?.track_ids ?? [])
        .map((id) => tracksById.get(id)!)
        .filter(Boolean);
    if (route.view === "artist")
      records = records.filter((t) => artist?.track_ids.includes(t.id));
    if(route.view==="playlist")records=(collections.find(c=>c.id===route.id)?.track_ids??[]).map(id=>tracksById.get(id)!).filter(Boolean);
    if (route.view === "favorites")
      records = records.filter((t) => t.favorited);
    return records.filter((t) => match(t.title, t.artist, t.album, t.genre));
  }, [library, route.view, route.id, needle,collections]);
  function albumItems(records: Album[]): CollectionItem[] {
    return records.map((a) => ({
      id: a.id,
      title: a.title,
      subtitle: a.artist,
      cover: a.cover,
      trackId: a.track_ids[0],
      playing:a.track_ids.includes(player.track?.id??-1),
      isPlaying:player.is_playing,
      levels:player.audio_levels,
      onClick: event => openAlbum(a,event?.currentTarget),
      menu: [
        {label:"Add to playlist…",icon:"Playlist",onSelect:()=>setPlaylistTrack({...library.tracks.find(t=>t.id===a.track_ids[0])!,selectionIds:a.track_ids})},
        {label:"Edit metadata",icon:"File",onSelect:()=>openManagement("review",a.track_ids)},
        { label: "Play", icon: "play", onSelect: () => playIds(a.track_ids) },
        {
          label: "Shuffle",
          icon: "ShufflePlay",
          onSelect: () => playIds(a.track_ids, true),
        },
        {
          label: "View Artist",
          icon: "Artist",
          onSelect: () => {
            const artist = library.artists.find((b) => b.id === a.artist_id);
            if (artist) openArtist(artist);
          },
        },
      ],
    }));
  }
  const selected = (id: string) =>
    dispatch({ type: "patch", patch: { selected: id } });
  const rows = (records: Track[]) => {const content=(
    <TrackRows
      tracks={records}
      showDurations={preferences.show_song_durations!==false}
      albumArtist={route.view==='album'?album?.artist:undefined}
      levels={player.audio_levels}
      scrollRef={scrollRef}
      scale={scale}
      initialOffset={route.scroll}
      playingId={player.track?.id}
      tempos={tempos}
      isPlaying={player.is_playing}
      selected={route.selected}
      onSelect={selected}
      onPlay={(t) =>
        playIds(
          records.map((t) => t.id),
          false,
          t.id,
        )
      }
      onToggle={()=>void run({type:"toggle"})}
      onArtist={artistFor}
      onAlbum={albumFor}
      onEdit={(track,ids)=>openManagement("review",ids??[track.id])}
      onFavorite={track=>void favorite(track)}
      playlistActions={(track,ids)=>[{label:"Play next",icon:"Play Next",disabled:!track.available,onSelect:()=>void run({type:"play_next",id:track.id})},{label:"Add to queue",icon:"Add to queue",disabled:!track.available,onSelect:()=>void run({type:"add_to_queue",id:track.id})},{label:"Add to playlist…",icon:"Playlist",onSelect:()=>setPlaylistTrack({...track,selectionIds:ids})}]}
    />
  );return route.view==='playlist'?content:<TrackSelection.Provider value={{ids:rowSelection,select:(id,index,event)=>{setRowSelection(old=>{if(event.shiftKey)return new Set(records.slice(Math.min(rowAnchor.current,index),Math.max(rowAnchor.current,index)+1).map(t=>t.id));rowAnchor.current=index;if(event.metaKey||event.ctrlKey){const next=new Set(old);next.has(id)?next.delete(id):next.add(id);return next;}return new Set([id]);});},remove:()=>{},selectAll:()=>setRowSelection(new Set(records.map(t=>t.id)))}}>{content}</TrackSelection.Provider>;};
  const grid = (items: CollectionItem[]) => (
    <CollectionGrid
      parallax={preferences.artwork_parallax!==false}
      depth={!!preferences.artwork_depth}
      items={items}
      scrollRef={scrollRef}
      scale={scale}
      initialOffset={route.scroll}
      selected={route.selected}
      onSelect={selected}
    />
  );
  const collectionActions = (ids: number[], artistAction?: () => void) => (
    <div className="collection-actions">
      <button
        className="play-button"
        aria-label="Play collection"
        onClick={() => playIds(ids)}
        disabled={!ids.some((id) => availableIds.has(id))}
      >
        <Icon name="play" />
        <span>Play</span>
      </button>
      <button
        className="shuffle-button"
        aria-label="Shuffle collection"
        onClick={() => playIds(ids, true)}
        disabled={!ids.some((id) => availableIds.has(id))}
      >
        <Icon name="CollectionShuffle" />
        <span>Shuffle</span>
      </button>
      <Overflow
        items={[
          {
            label: "Play",
            icon: "play",
            onSelect: () => playIds(ids),
            disabled: !ids.length,
          },
          {
            label: "Shuffle",
            icon: "ShufflePlay",
            onSelect: () => playIds(ids, true),
            disabled: !ids.length,
          },
          ...(artistAction
            ? [{ label: "View Artist", icon: "Artist", onSelect: artistAction }]
            : []),
        ]}
      />
    </div>
  );
  let content;
  if (loading)
    content = (
      <div className="loading" role="status">

      </div>
    );
  else if (route.view === "album" && album)
    content = (
      <article className="album-detail" data-album-id={album.id}>
        <header className="album-hero">
          <AlbumArtwork key={album.id} album={album} scrollRef={scrollRef} depth={!!preferences.artwork_depth} motion={preferences.artwork_parallax!==false}/>
          <div className="album-copy">
            <h1>{album.title}</h1>
            <AlbumArtists artists={library.artists.filter(a=>a.album_ids.includes(album.id))} primaryId={album.artist_id} loading={artistLoading} onOpen={openArtist}/>
            <p className="collection-meta">
              {album.year > 0 ? `${album.year} · ` : ""}
              {album.track_ids.length} songs ·{" "}
              {collectionDuration(album.duration)}
            </p>
            <AudioQuality key={album.id} tracks={tracks}/>
            {collectionActions(album.track_ids, () => {
              const a = library.artists.find((a) => a.id === album.artist_id);
              if (a) openArtist(a);
            })}
          </div>
        </header>
        <AlbumDescription key={album.id} albumId={album.id}/>
        {rows(tracks)}
      </article>
    );
  else if (route.view === "artist" && artist) {
    const albums = library.albums.filter((a) =>
      artist.album_ids.includes(a.id),
    );
    const fallbackAlbum = artistCoverAlbums.get(artist.id);
    content = (
      <article className="artist-detail" data-artist-id={artist.id}>
        <ArtistOverview key={artist.id} artist={artist} albums={albums} tracks={tracks} info={artistInfo}
          fallback={fallbackAlbum} actions={collectionActions(artist.track_ids)}
          onProvider={url=>void call("open_provider",{url}).catch(e=>setError(errorText(e)))} />
        <h2 className="section-title">Albums</h2>
        <div className="album-strip">
          {albumItems(albums).map((item) => (
            <button
              key={item.id}
              aria-label={item.title}
              className="artist-album-link" data-album-id={item.id}
              onClick={item.onClick}
            >
              <Cover
                src={item.cover}
                trackId={item.trackId}
                title={item.title}
              />
              <span>{item.title}</span>
            </button>
          ))}
        </div>
        <h2 className="section-title">Songs</h2>
        {rows(tracks)}
      </article>
    );
  } else if (route.view === "sources" || route.view === "review")
    content = (
      <LibraryTools
        view={route.view}
        library={library}
        busy={task.running}
        onAdd={(copy) => void chooseFolder(copy)}
        onTask={startTask}
        onRefresh={refresh}
        onError={setError}
      />
    );
  else if (
    route.view === "album" ||
    route.view === "artist" ||
    (route.view === "genre" && !library.genres.some((g) => g.id === route.id))
  )
    content = (
      <Empty
        title="This collection is no longer here."
        action={
          <button onClick={() => goScope("albums")}>Back to Albums</button>
        }
      >
        Rescan your music folders to update the library.
      </Empty>
    );
  else if(route.view==='sparklists'||route.view==='playlists') content=<CollectionManager library={library} items={collections} onChange={setCollections} onOpen={openCollection} initialTab={route.view==='sparklists'?'audio':'manual'} onCreate={setCreatingCollection} onSetup={()=>{sessionStorage.setItem('musicos-audio-page','spark');sessionStorage.setItem('musicos-settings-tab','Audio');setSection('Settings');}}/>;
  else if (!library.tracks.length && route.view!=='playlist')
    content = (
      <Empty
        title="Your library starts here."
        action={
          <button className="primary" onClick={() => void chooseFolder()}>
            Add a music folder
          </button>
        }
      >
        Bring your albums, artists, and favorite songs into MusicOS.
      </Empty>
    );
  else if(route.view==='playlist') {
    const item=collections.find(c=>c.id===route.id);
    const Detail=item?.kind==='audio'?AudioSparkDetail:PlaylistDetail;
    content=item?<Detail key={item.id} item={item} items={collections} library={library} onChange={setCollections} onDeleted={()=>navigation.history.length?goBack():goScope('albums')} scale={scale} visibleTrackIds={tracks.map(t=>t.id)} actions={collectionActions(tracks.map(t=>t.id))}>{rows(tracks)}</Detail>:<Empty title="Playlist unavailable">It may have been removed.</Empty>;
  }
  else if (route.view === "songs" || route.view === "favorites")
    content = tracks.length ? (
      rows(tracks)
    ) : (
      <Empty title="No songs found.">
        Try another search or add more music.
      </Empty>
    );
  else if (route.view === "artists")
    content = grid(
      (preferences.artists_album_only?albumArtists(library):library.artists)
        .filter((a) => match(a.name))
        .map((a) => ({
          id: a.id,
          title: a.name,
          subtitle: `${a.album_ids.length} album${a.album_ids.length===1?"":"s"} · ${a.track_ids.length} song${a.track_ids.length===1?"":"s"}`,
          cover: a.portrait,
          portrait: true,
          loading:artistLoading.has(a.name),
          loadingCover:artistCoverAlbums.get(a.id)?.cover,
          playing:a.track_ids.includes(player.track?.id??-1),
          isPlaying:player.is_playing,
      levels:player.audio_levels,
          onClick: event => openArtist(a,event?.currentTarget),
          menu: [
            {
              label: "Play",
              icon: "play",
              onSelect: () => playIds(a.track_ids),
            },
            {
              label: "Shuffle",
              icon: "ShufflePlay",
              onSelect: () => playIds(a.track_ids, true),
            },
          ],
        })),
    );
  else if (route.view === "genres")
    content = grid(
      library.genres
        .filter((g) => match(g.name))
        .map((g) => ({
          id: g.id,
          title: g.name,
          subtitle: `${g.album_ids.length} albums · ${g.track_ids.length} songs`,
          icon: "Genre",
          onClick: () =>
            openRoute({ view: "genre", id: g.id, search: "", scroll: 0 }),
        })),
    );
  else {
    const g =
      route.view === "genre"
        ? library.genres.find((g) => g.id === route.id)
        : null;
    const filters={...defaultFilters,...preferences.library_filters};
    const albums = filterAlbums(library,filters).filter(
      (a) =>
        (!g || g.album_ids.includes(a.id)) && match(a.title, a.artist, a.genre),
    ).sort((a,b) => filters.sort === "title" ? a.title.localeCompare(b.title) : b.track_ids.reduce((m,id) => Math.max(m,id),0) - a.track_ids.reduce((m,id) => Math.max(m,id),0));
    content = (
      <>
        {g && (
          <header className="genre-heading">
            <h1>{g.name}</h1>
            <p>
              {g.track_ids.length} songs · {g.album_ids.length} albums
            </p>
            {collectionActions(g.track_ids)}
          </header>
        )}
        {albums.length ? (
          filters.separate_singles&&route.view==='albums'?<><h2 className="section-title">Albums</h2>{grid(albumItems(albums.filter(a=>!a.release_type||!['single','ep'].includes(a.release_type))))}{albums.some(a=>a.release_type&&['single','ep'].includes(a.release_type))&&<><h2 className="section-title">Singles & EPs</h2>{grid(albumItems(albums.filter(a=>a.release_type&&['single','ep'].includes(a.release_type))))}</>}</>:grid(albumItems(albums))
        ) : (
          <Empty title="No albums found.">Try a different search.</Empty>
        )}
      </>
    );
  }
  async function changeMini(mode:'normal'|'tall'|'queue') {
    if(miniChanging.current)return;
    miniChanging.current=true;
    try {if(native)await call('mini_player',{mode,pinned:miniPinned});setMiniMode(mode);setQueueOpen(false);if(mode!=='normal')changePreferences({...preferences,mini_player_view:mode==='queue'?'queue':'artwork'});}
    catch(e){if(native)await call('mini_player',{mode:'normal'}).catch(()=>{});setMiniMode('normal');setError(errorText(e));}
    finally {miniChanging.current=false;}
  }
  async function toggleWindowPin(){
    const enabled=!(miniMode==='normal'?windowPinned:miniPinned);
    try{if(native)await call('window_pin',{enabled});if(miniMode==='normal')setWindowPinned(enabled);else setMiniPinned(enabled);}
    catch(e){setError(errorText(e));}
  }
  async function showPlaying(kind:'artist'|'album',name?:string) {
    if(miniMode!=='normal')await changeMini('normal');
    if(player.track){setSection('Library');if(kind==='artist')artistFor(player.track,name);else albumFor(player.track);}
  }
  async function showQueue(){if(miniMode!=='normal')await changeMini('normal');setQueueOpen(true);}
  const nowActions = player.track ? [
    {label:player.track.favorited?'Unfavorite song':'Favorite song',icon:player.track.favorited?'hearted':'noheart',onSelect:()=>favorite(player.track!)},
    {label:'Add to playlist…',icon:'Playlist',onSelect:()=>{void (async()=>{if(miniMode!=='normal')await changeMini('normal');setPlaylistTrack(player.track);})();}},
    {label:'Show artist',icon:'Artist',onSelect:()=>void showPlaying('artist')},
    {label:'Show album',icon:'Album',onSelect:()=>void showPlaying('album')},
    {label:'Edit song details',icon:'Song',onSelect:()=>{void (async()=>{if(miniMode!=='normal')await changeMini('normal');openManagement('review',[player.track!.id]);})();}},
  ] : [];

  const displayScope = (
    ["album", "artist", "genre"].includes(route.view)
      ? `${route.view}s`
      : route.view
  ) as Scope;
  return (<>
    {miniMode!=='normal'&&(<div className="app-shell mini-shell" style={{...playingStyle,background:"transparent"}} data-theme-mode={look.mode==='system'?(systemDark?'dark':'light'):look.mode}><MiniPlayer player={player} cover={playingCover} bpm={player.track?tempos.get(player.track.id):undefined} mode={miniMode} onMode={mode=>void changeMini(mode)} onAction={run} onFavorite={()=>{if(player.track)favorite(player.track);}} onAlbum={()=>void showPlaying('album')} onQueue={()=>void changeMini(miniMode==='queue'?'tall':'queue')} onMenuResize={async open=>{if(native)await call('mini_player',{mode:miniMode,menuOpen:open,pinned:miniPinned});}} pinned={miniPinned} onPin={()=>void toggleWindowPin()} onWindowError={setError} items={nowActions}/>{error&&<button className="mini-player-error" onClick={()=>{void changeMini('normal');}}>{error} · Return to MusicOS</button>}</div>)}
    <KeepView className="main-view" active={miniMode==='normal'}><div
      className="app-shell"
      style={{'--playing-favorite-tone':playingPalette?.accent??'var(--accent)'} as CSSProperties}
      data-size={preferences.interface_percent}
      data-adaptive={!!palette}
      data-theme-mode={look.mode==='system'?(systemDark?'dark':'light'):look.mode}
    >
      <a className="skip-link" href="#library-content">
        Skip to content
      </a>
      <header className="app-header" onMouseDown={e=>dragWindow(e,setError)}>
        <div
          className="window-drag-region"
        ><div className="header-brand"><Icon name="Album"/><span>MusicOS</span></div></div>
        <div className="section-control">
        <Menu
          label="Choose section"
          selectedLabel={section}
          onIntent={()=>{void import("./Settings");void import("./Devices");void import("./CollectionManager");}}
          items={(["Library", "Devices", "Settings"] as const).map((label) => ({
            label,
            onSelect: () => chooseSection(label),
          }))}
        >
          <span className="section-name">{section}</span>
          <span className="chevron" aria-hidden="true"><svg viewBox="0 0 24 12"><path d="M1 1 12 10 23 1" /></svg></span>
        </Menu>
        <Menu label="MusicOS actions" items={[{label:'Mini Player',icon:'openMiniplayer',onSelect:()=>void changeMini(preferences.mini_player_view==='queue'?'queue':'tall')},{label:'Manage collections',icon:'Playlist',onSelect:()=>{setSection('Library');goScope('playlists');}}]}><Icon name="3dots_(nocircle)"/></Menu>
        </div>
        <div className="header-drag-spacer" />
        <section style={playingStyle} className="now-playing" aria-label="Now playing" onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('.context-only')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} onKeyDown={e=>{if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('.context-only')?.dispatchEvent(new Event('musicos-open-menu'));}}}>
          <Overflow theme={playingStyle} contextOnly items={[{label:'Mini Player',icon:'openMiniplayer',onSelect:()=>void changeMini(preferences.mini_player_view==='queue'?'queue':'tall')},{label:windowPinned?'Unpin window':'Pin window',icon:windowPinned?'unPin':'Pin',onSelect:()=>void toggleWindowPin()},...nowActions]}/>
          <button
            className="now-playing-cover"
            aria-label={preferences.now_playing_artwork_action==='artwork'?"Show playing artwork":preferences.now_playing_artwork_action==='mini_player'?"Open Mini Player":"Show playing album"}
            onClick={event => {
              if(preferences.now_playing_artwork_action==='artwork'){openArtwork(event.currentTarget,player.track?.album??'Artwork');return;}
              if(preferences.now_playing_artwork_action==='mini_player'){void changeMini(preferences.mini_player_view==='queue'?'queue':'tall');return;}
              if (player.track) {
                setSection("Library");
                albumFor(player.track);
              }
            }}
            disabled={!player.track}
          >
            <Cover
              songArtwork
              src={playingCover}
              trackId={player.track?.id}
              title={player.track?.album ?? "No music playing"}
            />
          </button>
          <div className="now-playing-info">
            <strong className="now-playing-title" title={player.track?.title}>
              <OverflowLabel text={player.track?.title??(loading?'':'Nothing playing yet')}/>
              {player.track&&<FavoriteButton track={player.track} bpm={tempos.get(player.track.id)} playing={player.is_playing} onFavorite={favorite} className="now-playing-heart" label={player.track.favorited?'Unfavorite playing song':'Favorite playing song'}/>}
            </strong>
            <span className="now-playing-credits">{player.track?<><span className="now-playing-artists">{trackCredits(player.track).map((name,index)=><span key={name}>{index>0&&<i aria-hidden="true"> + </i>}<button onClick={()=>void showPlaying('artist',name)} title={name}>{name}</button></span>)}</span><i aria-hidden="true"> · </i><button className="now-playing-album" onClick={()=>void showPlaying('album')} title={player.track.album}><OverflowLabel text={player.track.album}/></button></>:loading?"":"Your next favorite is waiting."}</span>
            <PlaybackProgress showTime={!loading&&preferences.show_now_playing_time!==false} transition={player.dj_transition} trackId={player.track?.id} position={player.position}
              duration={player.track?.duration ?? 0}
              seek={async position => { setPlayer(await call("playback", {action:{type:"seek",position}})); }} />
          </div>
          <div className="mini-controls">
            <IconButton name="queue" label="Open Up next" pressed={queueOpen} onClick={() => setQueueOpen(!queueOpen)} />
            <AudioQuality key={player.track?.id??'empty'} tracks={player.track?[player.track]:[]} compact/>
            <IconButton
              name={player.shuffle ? "ShuffleON" : "ShuffleOFF"}
              label="Shuffle"
              pressed={player.shuffle}
              onClick={() => void run({ type: "shuffle" })}
              disabled={!player.track}
            />
            <IconButton
              name={
                player.repeat === "one"
                  ? "RepeatONE"
                  : player.repeat === "all"
                    ? "RepeatON"
                    : "RepeatOFF"
              }
              label="Repeat"
              onClick={() => void run({ type: "repeat" })}
              disabled={!player.track}
            />
          </div>
          <div className="transport">
            <IconButton
              name="prev"
              label="Previous track"
              disabled={!player.track}
              onClick={() => void run({ type: "previous" })}
            />
            <IconButton
              name={player.is_playing ? "pause" : "play"}
              label={player.is_playing ? "Pause" : "Play"}
              disabled={!player.track}
              onClick={() => void run({ type: "toggle" })}
            />
            <IconButton
              name="next"
              label="Next track"
              disabled={!player.track}
              onClick={() => void run({ type: "next" })}
            />
          </div>

        </section>
      </header>
      <div className={`workspace ${section !== "Library" ? "full-width" : ""}`}>
        {section === "Library" && (
          <aside className="library-sidebar">
            <span className="sidebar-label">Library</span>
            <div className="sidebar-search-slot">
              {navigation.history.length>0?<button className="sidebar-back" onClick={goBack}><Icon name="Back"/><span>Back to {(() => {const view=navigation.history.at(-1)!.view;return ['album','artist','genre'].includes(view)?view:view[0].toUpperCase()+view.slice(1);})()}</span></button>:<button className="search sidebar-search" aria-label="Search music and commands" aria-keyshortcuts="Meta+K Control+K" onClick={()=>setCommandOpen(true)}><Icon name="search"/><span>Search</span><kbd>⌘ K</kbd></button>}
              {navigation.history.length>0&&<button className="command-shortcut" aria-label="Open SuperCommand" title="SuperCommand (⌘K / Ctrl+K)" onClick={()=>setCommandOpen(true)}><span aria-hidden="true" className="command-symbol">⌘</span></button>}
            </div>
            <nav aria-label="Library">
              {(["albums", "artists", "genres", "songs"] as Scope[]).map(
                (scope) => (
                  <button
                    className={displayScope === scope ? "active" : ""}
                    key={scope}
                    onClick={() => goScope(scope)}
                  >
                    <Icon name={scopeIcons[scope]} />
                    <span>{scope[0].toUpperCase()+scope.slice(1)}</span>
                  </button>
                ),
              )}
            </nav>
            <div className="collection-create-actions">
              <button className="sketch-button" onClick={()=>setCreatingCollection('manual')}><Icon name="Playlist"/><span>Create playlist</span></button>
              <button className="sketch-button secondary" onClick={()=>setCreatingCollection('spark')}><Icon name="Sparklist"/><span>Create Sparklist</span></button>
            </div>
            <section className="personal-library">
              <div className="collections-heading"><h2><button className="collections-heading-link" aria-label="Manage collections" onClick={()=>{setSection('Library');goScope('playlists');}}>{preferences.display_name}'s stuff</button></h2></div>
              <button
                className={route.view === "favorites" ? "active" : ""}
                onClick={() => goScope("favorites")}
              >
                <Icon name="hearted" />
                <span>
                  <strong>Favorites</strong>
                  <small aria-busy={loading}>
                    {loading?'\u00a0':`${librarySummary.favorites} songs`}
                  </small>
                </span>
              </button>
              <div className="personal-collections">{collections.map(item=>{
                const summary=collectionSummaries.get(item.id)!;
                const icon=item.kind==='audio'?'Sparklist':'Playlist';
                return <div key={item.id} data-manual-collection={item.kind!=='audio'&&!item.spark?item.id:undefined} className={`personal-collection ${route.id===item.id?'active':''}`}>
                  <button className="collection-open" onClick={()=>openCollection(item)}><PlaylistArtwork item={item} library={library}/><span className="collection-label"><strong><span>{item.name}</span><Icon name={icon}/></strong><small>{item.kind==='rule'?'Smart playlist · ':''}{item.track_ids.length} {item.track_ids.length===1?'song':'songs'} · {collectionDuration(summary.duration)}</small></span></button>
                  <button className="collection-shuffle" aria-label={`Shuffle ${item.name}`} disabled={!summary.available} onClick={()=>playIds(item.track_ids,true)}><Icon name="CollectionShuffle"/></button>
                </div>;
              })}</div>
              {collectionError&&<small role="alert">{collectionError}</small>}
            </section>
            <footer className="sidebar-footer">
              <span aria-busy={loading}>{loading?"":`${library.tracks.length} songs`}</span>
              <span>
                {loading?"":collectionDuration(librarySummary.duration)}
              </span>
            </footer>
          </aside>
        )}
        <div className="content-column">
          {section === "Library" &&
            !["album", "artist"].includes(route.view) && (
              <div className="library-toolbar">
                {navigation.history.length > 0 && (
                  <IconButton
                    name="Back"
                    label="Back"
                    onClick={() => goBack()}
                  />
                )}
                {route.view === "albums" ? <LibraryFilter library={library} value={{...defaultFilters,...preferences.library_filters}} onChange={library_filters=>changePreferences({...preferences,library_filters})}/> : <span className="toolbar-title">
                  {route.view === "album"
                    ? album?.title
                    : route.view === "artist"
                      ? artist?.name
                      : route.view === "genre"
                        ? library.genres.find((g) => g.id === route.id)?.name
                        : route.view === "sources"
                          ? "Music folders"
                          : route.view === "review"
                            ? "Metadata & artwork"
                            : route.view === "playlists" || route.view === "sparklists"
                              ? "Collections"
                              : route.view === "catalog"
                              ? "Discover"
                              : route.view[0].toUpperCase() +
                                route.view.slice(1)}
                </span>}
                {route.view==="artists"&&<ThemedSwitch label="Album Artists" checked={!!preferences.artists_album_only} onChange={artists_album_only=>changePreferences({...preferences,artists_album_only})}/>}
              </div>
            )}
          <div className="page-frame">
            <div className="theme-layer current-theme" aria-hidden="true"/>
            <main
              id="library-content"
              className={`page-scroll ${section.toLowerCase()}-page`}
              ref={scrollRef}
            >
            <div className="page-content">
              <KeepView active={section === "Settings"}><Suspense fallback={<div className="screen-placeholder" aria-label="Loading settings" aria-busy="true"/>}>
                <Settings
                  preferences={preferences}
                  onChange={changePreferences}
                  error={saveError}
                  cover={playingCover ?? resolvedThemeCover}
                  playback={player}
                  onVolume={volume=>run({type:"volume",volume})}
                  palette={palette}
                  audioContent={<AudioAnalysis library={library}/>}
                  libraryView={settingsLibraryView}
                  onLibraryView={view=>{setSettingsLibraryView(view);setMetadataIds([]);}}
                  onManageCollections={()=>{setSection("Library");goScope("playlists");}}
                  libraryContent={<LibraryTools view={settingsLibraryView==='review'?'review':'sources'} initialIds={metadataIds} library={library} busy={task.running} onAdd={copy=>void chooseFolder(copy)} onTask={startTask} onRefresh={refresh} onError={setError}/>}

                />
              </Suspense></KeepView><KeepView className="library-view" active={section==='Library'}><Suspense fallback={<div className="screen-placeholder" aria-label="Loading library view" aria-busy="true"/>}>{content}</Suspense></KeepView>{section === "Devices" ? (<Suspense fallback={<div className="screen-placeholder" aria-label="Loading devices" aria-busy="true"/>}>
                <Devices
                  busy={task.running}
                  onSync={(root) => startTask("sync", root)}
                />
              </Suspense>) : null}
            </div>
          </main>
          </div>
          {task.message && (
            <div
              className={`task-status ${task.error ? "has-error" : ""}`}
              role="status"
            >
              <span>
                <strong>
                  {task.running
                    ? `${task.kind[0]?.toUpperCase()}${task.kind.slice(1)}`
                    : task.cancelled
                      ? "Cancelled"
                      : task.error
                        ? "Could not finish"
                        : "Finished"}
                </strong>{" "}
                · {task.error ?? task.message}
              </span>
              {task.running && (
                <>
                  <progress
                    value={task.total ? task.processed : undefined}
                    max={Math.max(1, task.total)}
                  />
                  <button onClick={() => void call("cancel_task")}>
                    Cancel
                  </button>
                </>
              )}
              {!task.running && (
                <button
                  aria-label="Dismiss task"
                  onClick={e => {
                    const surface=e.currentTarget.closest('.task-status');
                    const clear=()=>setTask(current=>current===task?emptyTask:current);
                    if(surface&&!window.matchMedia('(prefers-reduced-motion: reduce)').matches)void surface.animate([{opacity:1},{opacity:0,transform:'translateY(.25rem)'}],{duration:150,easing:'ease-in',fill:'forwards'}).finished.then(clear).catch(()=>{});
                    else clear();
                  }}
                >
                  ×
                </button>
              )}
            </div>
          )}
        </div>
      </div>
      {fileDrop&&<div className="file-drop-hint" role="status"><Icon name="Song"/><strong>Copy music into MusicOS</strong><span>Drop songs or folders. Originals stay where they are.</span></div>}
      {playlistTrack&&<PlaylistPicker ids={(playlistTrack as Track&{selectionIds?:number[]}).selectionIds} track={playlistTrack} library={library} items={collections} onChange={setCollections} onClose={()=>setPlaylistTrack(null)}/>}
      {creatingCollection&&<CollectionCreate kind={creatingCollection} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)} sparkContent={<Suspense fallback={<div className="screen-placeholder" aria-label="Loading Sparklist" aria-busy="true"/>}><SparkCreate preferences={preferences} onPreferences={changePreferences} playingId={player.track?.id} isPlaying={player.is_playing} onToggle={async()=>setPlayer(await call("playback",{action:{type:"toggle"}}))} onPlay={async ids=>{const playable=ids.filter(id=>availableIds.has(id));if(!playable.length)throw Error("No available preview songs.");setPlayer(await call("playback",{action:{type:"play",ids:playable,start_index:0,shuffle:false}}));}} onSetup={()=>{sessionStorage.setItem("musicos-audio-page","spark");window.dispatchEvent(new CustomEvent("musicos-audio-page",{detail:"spark"}));setCreatingCollection(null);captureScroll();sessionStorage.setItem("musicos-settings-tab","Audio");window.dispatchEvent(new CustomEvent("musicos-settings-tab",{detail:"Audio"}));setSection("Settings");}} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)}/></Suspense>}/>}
      {pendingImport && (
        <dialog
          ref={importDialog}
          className="import-dialog"
          onCancel={() => setPendingImport(null)}
        >
          <h2>Add your music</h2>
          <p>
            {pendingImport.paths.length}{" "}
            {pendingImport.folders ? "folder(s)" : "file(s) or folders"}{" "}
            selected.
          </p>
          <p>
            {pendingImport.folders
              ? "Keep music in its current location, or make a copy in your MusicOS library."
              : "Copy these files into your managed MusicOS library. The originals stay where they are."}
          </p>
          <div className="tool-actions">
            {pendingImport.folders && (
              <button
                autoFocus
                className="primary"
                disabled={task.running}
                onClick={() => {
                  void call("library_task", {
                    request: { kind: "reference", paths: pendingImport.paths },
                  }).catch((e) => setError(errorText(e)));
                  setPendingImport(null);
                }}
              >
                Keep in place
              </button>
            )}
            <button
              autoFocus={!pendingImport.folders}
              disabled={task.running}
              onClick={() => {
                void call("library_task", {
                  request: { kind: "copy", paths: pendingImport.paths },
                }).catch((e) => setError(errorText(e)));
                setPendingImport(null);
              }}
            >
              Copy into MusicOS
            </button>
            <button onClick={() => setPendingImport(null)}>Cancel</button>
          </div>
        </dialog>
      )}
      <QuickAudio theme={playingStyle} player={player} preferences={preferences} onChange={changePreferences} onVolume={volume=>void run({type:"volume",volume})} error={saveError}/>
      <ArtworkViewer depth={!!preferences.artwork_depth}/>
      {queuePresent&&<AudioPanel theme={playingStyle} closing={!queueOpen} player={player} scale={scale} presentation={preferences.audio_presentation??'sidebar'} onClose={()=>setQueueOpen(false)} onAction={action=>void run(action)} onSettings={()=>{sessionStorage.setItem('musicos-settings-tab','Audio');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));setQueueOpen(false);setSection('Settings');}}/>}
      {commandOpen&&<Suspense fallback={null}><SuperCommand library={library} collections={collections} isPlaying={player.is_playing} hasTrack={!!player.track} onClose={()=>setCommandOpen(false)} onPlay={id=>playIds([id])} onAlbum={a=>{setSection('Library');openAlbum(a);}} onArtist={a=>{setSection('Library');openArtist(a);}} onCollection={openCollection} onNavigate={destination=>{if(destination==='Settings'||destination==='Devices')chooseSection(destination);else{setSection('Library');goScope(destination.toLowerCase() as Route['view']);}}} onPlayback={type=>void run({type})} onQueue={()=>setQueueOpen(true)} onCreate={setCollections}/></Suspense>}
      {(error || visiblePlaybackError) && (
        <div className="error-toast" role="alert">
          <span>{error || visiblePlaybackError}</span>
          <button
            aria-label="Dismiss error"
            onClick={() => {
              setError("");
              setDismissedPlaybackError(player.error);
            }}
          >
            ×
          </button>
        </div>
      )}
      {!native && !new URLSearchParams(location.search).has("fixture") && (
        <div className="preview-notice">
          Browser preview · Open the desktop app to use your music.
        </div>
      )}
    </div></KeepView></>
  );
}
