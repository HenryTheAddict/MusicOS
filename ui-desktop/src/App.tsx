import {useScrollEdges} from './useScrollEdges';
import Activity from './Activity';
import {SongPreviewProvider} from './SongPreview';
import {installController,defaultController} from './controller';
import {useNativePlayerShortcuts,type PlayerShortcut} from './useNativePlayerShortcuts';
import VisualizerPlayer from './VisualizerPlayer';
import {defaultVisualizer,playerShortcut} from './visualizer';
import {nextPlayerView} from './PlayerViewCycle';
import PlayerViewCycle from './PlayerViewCycle';
import {setFeedbackSettings,installInteractionFeedback} from './haptics';
import{usePlayerHandoff}from'./usePlayerHandoff';
import type {MetadataEntry} from './MetadataWorkspace';
import {captureBackPage,useBackSwipe,type BackArtwork} from './useBackSwipe';
import Onboarding from './Onboarding';
import {product} from './product';
import {dropImportSource} from './importDestination';
import {usePlayfulControls} from './playfulControls';
import {useShellLayout} from './useShellLayout';
import {useDestinationNavigation,type Destination} from './useDestinationNavigation';
import ExpandedPlayer,{type PlayerView} from './ExpandedPlayer';
import {observeInputModality} from './inputModality';
import {launchPreferences,rememberLaunchAppearance,rememberLaunchPalette,savedLaunchPalette} from './launchTheme';
import {usePlaylistDrop} from './usePlaylistDrop';
import {clockPosition,currentPlaybackClock,usePlaybackFeed} from './playbackFeed';
import {animateOwned} from './presentationMotion';
import {secondaryInk} from './appearance';
import {personalName} from './personalName';
import ListFind from './ListFind';
import {compileListFind} from './findQuery';
import {useAutomaticSparkModel} from './sparkModel';
import ActionSurface from './ActionSurface';
import useSparkRecommendations from './useSparkRecommendations';
import FavoriteButton from './FavoriteButton';
import OverflowLabel from './OverflowLabel';
import {playerTheme} from './playerTheme';
import {invalidateDisplay,type LibraryChanged} from './displayData';
import {KeepView,ViewActivity} from './ViewScope';
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
import {SmartCreate} from "./SmartEditor";
import {CollectionDelete} from "./CollectionActions";
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
import {PlaylistDetail,PlaylistArtwork,AudioSparkDetail,playlistFile} from './PlaylistLibrary';
const CollectionManager = lazy(()=>import("./CollectionManager"));
const AudioAnalysis = lazy(()=>import("./AudioAnalysis"));
const Settings = lazy(()=>import("./Settings"));
import ArtistOverview from "./ArtistOverview";
import { resolveArtworkTheme, clearThemeCache } from "./artworkTheme";
import { useNativeChrome } from "./nativeChrome";
const Devices = lazy(()=>import("./Devices"));
const LibraryTools = lazy(()=>import("./LibraryTools"));
import { usePageEntrance } from "./motion";
import { useSmoothCorners,useWindowContour } from "./smoothCorners";
import {queuedMixForCurrent} from './playbackPresentation';
import PlaybackProgress from "./PlaybackProgress";
import AudioQuality from "./AudioQuality";
import AlbumDescription from "./AlbumDescription";
const PersonalHome = lazy(()=>import("./PersonalHome"));
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
const profileInfoCache=new Map<string,ArtistInfo>();
function rememberArtistInfo(name:string,info:ArtistInfo){if(profileInfoCache.size>=256&&!profileInfoCache.has(name))profileInfoCache.delete(profileInfoCache.keys().next().value!);profileInfoCache.set(name,info);}
function contrastInk(hex:string){
 const rgb=hex.replace('#','').match(/.{2}/g)?.map(v=>parseInt(v,16)/255);
 if(!rgb||rgb.length!==3)return '#fff';
 const linear=rgb.map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4);
 return linear[0]*.2126+linear[1]*.7152+linear[2]*.0722>.179?'#111820':'#fff';
}
export default function App() {
  usePlayfulControls();
  const [activityOpen,setActivityOpen]=useState(false);
  const [settingsLibraryView,setSettingsLibraryView]=useState<"sources"|"review"|"playlists"|"sparklists">("sources");
  const [metadataEntry,setMetadataEntry]=useState<MetadataEntry>();
  function openMetadata(entry?:MetadataEntry) {
    captureScroll();
    setSettingsLibraryView('review');
    setMetadataEntry(entry);
    sessionStorage.setItem('musicos-settings-tab','Library');
    window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Library'}));
    setSection('Settings');
  }
  useEffect(()=>{
    const edit=(event:Event)=>{
      const trackId=(event as CustomEvent<number>).detail;
      setExpandedOpen(false);
      setQueueOpen(false);
      if(miniMode!=='normal')void changeMini('normal');
      openMetadata({mode:'lyrics',trackId});
    };
    window.addEventListener('musicos-open-lyric-editor',edit);
    return()=>window.removeEventListener('musicos-open-lyric-editor',edit);
  });
  const [collections,setCollections]=useState<SavedCollection[]>([]);
  const [rowSelection,setRowSelection]=useState<Set<number>>(new Set());
  const rowAnchor=useRef(0);
  const [playlistTrack,setPlaylistTrack]=useState<(Track&{selectionIds?:number[]})|null>(null);
  const [collectionEdit,setCollectionEdit]=useState<{id:string;revision:number}|null>(null);
  const [deletingCollection,setDeletingCollection]=useState<SavedCollection|null>(null);
  const [sparkSeed,setSparkSeed]=useState<number|undefined>();
  function createCollection(kind:"manual"|"spark"|"rule"){setSparkSeed(undefined);setCreatingCollection(kind);}
  const [creatingCollection,setCreatingCollection]=useState<"manual"|"spark"|"rule"|null>(null);
  const [collectionError,setCollectionError]=useState("");

  const favoritePending=useRef(new Map<number,Promise<void>>());

  const [library, setRawLibrary] = useState<Library>(emptyLibrary);
  const setLibrary:typeof setRawLibrary=next=>setRawLibrary(old=>{const value=typeof next==='function'?next(old):next;registerAlbumArtwork(value);return value;});
  const dropTarget=useRef(dropImportSource(library));dropTarget.current=dropImportSource(library);
  const tracksById=useMemo(()=>new Map(library.tracks.map(t=>[t.id,t])),[library.tracks]);
  const librarySummary=useMemo(()=>({favorites:library.tracks.filter(t=>t.favorited).length,duration:library.tracks.reduce((sum,t)=>sum+t.duration,0)}),[library.tracks]);
  const dropNotice=usePlaylistDrop(collections,library.tracks,setCollections,setCollectionError);
  const collectionSummaries=useMemo(()=>new Map(collections.map(item=>{let duration=0,available=0;for(const id of item.track_ids){const track=tracksById.get(id);if(track){duration+=track.duration;available++;}}return [item.id,{duration,available}];})),[collections,tracksById]);
  const [preferences, setPreferences] = useState(()=>launchPreferences(defaultPreferences));
  useEffect(()=>setFeedbackSettings(preferences.trackpad_feedback),[preferences.trackpad_feedback]);
  useEffect(installInteractionFeedback,[]);
  const savedSparkBeta=useRef(defaultPreferences.sparklist_beta);
  useSmoothCorners();
  useWindowContour(native&&/Mac/.test(navigator.platform));
  const [miniMode,setMiniMode]=useState<'normal'|'tall'|'queue'>('normal');
  const miniChanging=useRef(false);
  const [miniPinned,setMiniPinned]=useState(true),[windowPinned,setWindowPinned]=useState(false);
  useNativeChrome(miniMode==='normal');
  const [rawPlayer, setPlayer] = usePlaybackFeed(emptyPlayback);
  const [homePlaybackStarted,setHomePlaybackStarted]=useState(false);
  useEffect(()=>{if(rawPlayer.is_playing)setHomePlaybackStarted(true);},[rawPlayer.is_playing]);
  const activitySong=useRef(rawPlayer.track?.id);
  useEffect(()=>{if(activitySong.current!==rawPlayer.track?.id){activitySong.current=rawPlayer.track?.id;invalidateDisplay('home_activity');}},[rawPlayer.track?.id]);
  const presentedQueue=useMemo(()=>rawPlayer.queue.map(t=>tracksById.get(t.id)??t),[rawPlayer.queue,tracksById]);
  const player=useMemo(()=>({...rawPlayer,track:rawPlayer.track?(tracksById.get(rawPlayer.track.id)??rawPlayer.track):null,queue:presentedQueue}),[rawPlayer,tracksById,presentedQueue]);
  const [task, setTask] = useState<Task>(emptyTask);
  const [loading, setLoading] = useState(true);
  const [onboardingOpen,setOnboardingOpen]=useState(false);
  const onboardingDecided=useRef(false),bootstrapSucceeded=useRef(false),librarySucceeded=useRef(false);
  useEffect(()=>{if(loading||onboardingDecided.current||!bootstrapSucceeded.current||!librarySucceeded.current)return;onboardingDecided.current=true;if((preferences.onboarding_version??1)<product.onboarding.version&&library.tracks.length===0&&!player.is_playing)setOnboardingOpen(true);},[loading,preferences.onboarding_version,library.tracks.length,player.is_playing]);
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
  const [queueView,setQueueView]=useState<'queue'|'lyrics'>('queue');
  const queuePresent=usePanelPresence(queueOpen);
  useEffect(()=>{const open=(event:Event)=>{const page=(event as CustomEvent<{page?:string}>).detail?.page==='spark'?'spark':'quality';setQueueOpen(false);sessionStorage.setItem('musicos-settings-tab','Audio');sessionStorage.setItem('musicos-audio-page',page);setSection('Settings');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));window.dispatchEvent(new CustomEvent('musicos-audio-page',{detail:page}));};window.addEventListener('musicos-open-audio-settings',open);return()=>window.removeEventListener('musicos-open-audio-settings',open);},[]);
  const [commandOpen,setCommandOpen]=useState(false);
  const commandPresent=usePanelPresence(commandOpen);
  const automaticModel=useAutomaticSparkModel(!!preferences.sparklist_beta,!loading&&miniMode==='normal'&&section==='Library');
  const sparkRecommendations=useSparkRecommendations(collections,player.track?.id,!loading&&miniMode==='normal'&&(section==='Library'||commandOpen),library,preferences.sparklist_beta);
  const [artistLoading,setArtistLoading]=useState<Set<string>>(()=>new Set());
  const artistCompletions=useRef(new Map<string,number>());
  const [, setArtistInfo] = useState<ArtistInfo | null>(null);
  useEffect(()=>{if(loading)return;const warm=()=>{void import('./Settings');void import('./CollectionManager');void import('./SuperCommand');void import('./LibraryTools');if(native)void call('library_ready').catch(()=>{});};const id=window.setTimeout(warm,1200);return()=>clearTimeout(id);},[loading]);
  const [themeEpoch, setThemeEpoch] = useState(0);
  useEffect(() => { const clear = () => { clearThemeCache(); setThemeEpoch(n=>n+1); }; window.addEventListener("musicos-artwork-invalidated", clear); return () => window.removeEventListener("musicos-artwork-invalidated", clear); }, []);
  const [resolvedArtwork,setResolvedArtwork]=useState<BackArtwork|null>(null);
  const artPalette=resolvedArtwork?.result?.palette??null;
  const resolvedThemeCover=resolvedArtwork?.result?.path??null;
  const [systemDark,setSystemDark]=useState(()=>matchMedia('(prefers-color-scheme: dark)').matches);
  useEffect(()=>{const media=matchMedia('(prefers-color-scheme: dark)');const change=()=>setSystemDark(media.matches);media.addEventListener('change',change);return()=>media.removeEventListener('change',change);},[]);
  const look=appearance(preferences);
  useEffect(()=>{if(!loading)rememberLaunchAppearance(look);},[loading,look.mode,look.source,look.accent,look.saturation,look.tint]);
  useEffect(()=>{document.documentElement.dataset.themeMode=look.mode==='system'?(systemDark?'dark':'light'):look.mode;},[look.mode,systemDark]);
  // Keep the launch fallback stable when no artwork is available. The launch
  // snapshot written by another page is for the next startup.
  const launchFallback=useMemo(()=>savedLaunchPalette(look,look.mode==='dark'||look.mode==='system'&&systemDark),[look.mode,look.source,look.accent,look.saturation,look.tint,systemDark]);
  const palette=useMemo(()=>!artPalette&&look.source==='artwork'&&launchFallback?launchFallback:themePalette(look,artPalette,systemDark),[look.mode,look.source,look.accent,look.saturation,look.tint,artPalette,systemDark,launchFallback]);
  useLayoutEffect(()=>{document.documentElement.dataset.playerLayout=miniMode;return()=>{delete document.documentElement.dataset.playerLayout;};},[miniMode]);
  const launchShown=useRef(false);
  useLayoutEffect(()=>{if(loading||launchShown.current)return;const node=document.querySelector<HTMLElement>('.app-shell:not(.mini-shell)');if(!node)return;launchShown.current=true;let alive=true,cancel:(()=>void)|undefined;const reveal=()=>{if(alive)cancel=animateOwned(node,'shell-launch',[{opacity:0,transform:'translateY(4px)'},{opacity:1,transform:'none'}],180);};if(native)void call('present_shell',{color:getComputedStyle(document.documentElement).getPropertyValue('--shell-color').trim()}).then(reveal).catch(reveal);else reveal();return()=>{alive=false;cancel?.();};},[loading]);
  useEffect(()=>{if(loading||look.source==='artwork'&&!artPalette)return;const resolved=palette??{background:'#cbf1ff',blob:'#00baff',accent:'#0064df',text:'#111820'};const timer=window.setTimeout(()=>{const snapshot=rememberLaunchPalette(look,resolved,look.mode==='dark'||look.mode==='system'&&systemDark);if(native)void call('save_launch_palette',{snapshot}).catch(()=>{});},180);return()=>clearTimeout(timer);},[loading,palette,systemDark,look.mode,look.source,look.accent,look.tint,look.saturation]);
  const launchThemeSettled=useRef(false);
  const themeResolved=resolvedArtwork!==null;
  useLayoutEffect(()=>{document.documentElement.dataset.themePhase="launch";return()=>{delete document.documentElement.dataset.themePhase;delete document.documentElement.dataset.themeMotion;};},[]);
  useEffect(()=>{
    if(launchThemeSettled.current||!themeResolved)return;
    const timer=setTimeout(()=>{launchThemeSettled.current=true;document.documentElement.dataset.themePhase="ready";},180);
    return()=>clearTimeout(timer);
  },[themeResolved]);
  const appliedTheme=useRef<{appearance:string;resolved:boolean;palette:Palette|null}|null>(null);
  const appearanceKey=JSON.stringify([look,look.mode==='system'?systemDark:null]);
  const swipeAppearanceKey=look.source==='artwork'?JSON.stringify([appearanceKey,themeEpoch]):'';
  useLayoutEffect(()=>{
    const previous=appliedTheme.current;
    if(previous?.appearance!==appearanceKey||look.source!=='artwork'){
      delete document.documentElement.dataset.themeMotion;
      if(previous?.resolved&&previous.appearance!==appearanceKey){launchThemeSettled.current=true;document.documentElement.dataset.themePhase='ready';}
    }
    else if(previous.resolved&&themeResolved&&previous.palette!==palette)document.documentElement.dataset.themeMotion='artwork';
    const tokens={'--page-color':palette?.background??(section==='Library'?'#00aeff':'#c9efff'),'--shell-color':palette?.blob??'#00baff','--accent':palette?.accent??'#0064df','--surface-color':palette?.background??'#cbf1ff','--blob-color':palette?.blob??'#54b9ff','--ink':palette?.text??'#111820','--accent-ink':look.source==='artwork'?(look.mode==='dark'||(look.mode==='system'&&systemDark)?'#000000':'#ffffff'):readableInk(palette?.accent??'#0064df')};
    document.documentElement.style.setProperty('--secondary-ink',secondaryInk(tokens['--ink'],tokens['--surface-color'],tokens['--blob-color'],look.mode==='dark'||(look.mode==='system'&&systemDark)));
    document.documentElement.style.setProperty('--playback-tone',tokens['--ink']);
    for(const [name,value] of Object.entries(tokens))document.documentElement.style.setProperty(name,value);
    appliedTheme.current={appearance:appearanceKey,resolved:themeResolved,palette};
  },[palette,section,themeResolved,appearanceKey]);
  const {navigation,dispatch,destination,switchDestination,scope:navigateScope}=useDestinationNavigation();
  const previousDestination=useRef<Destination>('home');
  const [searchFocus,setSearchFocus]=useState(0);
  const route = navigation.current;
  const findRoute=`${route.view}:${route.id??''}`;
  const findSupported=['albums','songs','favorites','playlist','genres','genre'].includes(route.view);
  const collectionFind=['playlists','sparklists'].includes(route.view);
  const [findKey,setFindKey]=useState<string|null>(null),[findRequest,setFindRequest]=useState(0);
  const findOpen=findSupported&&(findKey===findRoute||!!route.search);
  const findOrigins=useRef(new Map<string,number>()),findRouteRef=useRef(findRoute);findRouteRef.current=findRoute;
  function openFind(){
   if(collectionFind){setFindKey(findRoute);setFindRequest(value=>value+1);return;}
   if(!findSupported)return;
   if(!findOpen)findOrigins.current.set(findRoute,scrollRef.current?.scrollTop??route.scroll);
   setQueueOpen(false);setFindKey(findRoute);setFindRequest(value=>value+1);
  }
  function changeFind(search:string){dispatch({type:'patch',patch:{search,scroll:0}});if(scrollRef.current)scrollRef.current.scrollTop=0;}
  function closeFind(){const key=findRoute,scroll=findOrigins.current.get(key)??0;findOrigins.current.delete(key);setFindKey(null);dispatch({type:'patch',patch:{search:'',scroll}});requestAnimationFrame(()=>{if(findRouteRef.current!==key)return;if(scrollRef.current)scrollRef.current.scrollTop=scroll;document.querySelector<HTMLButtonElement>('.library-find-action')?.focus({preventScroll:true});});}

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

  usePageEntrance(scrollRef, `${section}:${route.view}:${route.id ?? ""}:${destination}`, !loading&&miniMode==='normal');
  const savePending = useRef<Preferences|null>(null);
  const savingPreferences=useRef(false);
  const savePromise=useRef<Promise<boolean>>(Promise.resolve(true));
  const scale = preferences.interface_percent / 100;
  const shell=useShellLayout(scale,miniMode==='normal');
  const compact=shell.compact;
  const [lyricPreview,setLyricPreview]=useState<Track|null>(null);
  const [visualizerOpen,setVisualizerOpen]=useState(false),[visualizerView,setVisualizerView]=useState<PlayerView>('playing');
  const visualizerOpener=useRef<HTMLElement|null>(null);
  const [expandedOpen,setExpandedOpen]=useState(false),[playerView,setPlayerView]=useState<PlayerView>('playing');
  const expandedPresent=usePanelPresence(expandedOpen);
  const audioPanelPresent=queuePresent&&!expandedPresent;
  const dockedAudio=audioPanelPresent&&!compact&&preferences.audio_presentation!=='floating'&&
    (section==='Library'?shell.dockLibrary:shell.dockSection);

  const backSwipeCue=useBackSwipe(shell.root,section==='Library'&&miniMode==='normal'&&!onboardingOpen&&!commandOpen&&!expandedOpen&&!visualizerOpen,navigation.history.length>0,goBack,navigation.history.at(-1)?JSON.stringify({...navigation.history.at(-1),scroll:0}):'',swipeAppearanceKey);
  const playerOrigin=useRef<DOMRect|null>(null),playerOpener=useRef<HTMLElement|null>(null);
  function openPlayer(view:PlayerView='playing',preview:Track|null=null){setLyricPreview(preview);playerOpener.current=document.querySelector<HTMLElement>(view==='lyrics'?'.now-playing-title-action':view==='queue'?'.now-playing [aria-label="Open Up next"]':'.now-playing-cover');playerOrigin.current=document.querySelector('.now-playing-cover .cover')?.getBoundingClientRect()??null;setQueueOpen(false);setPlayerView(view);setExpandedOpen(true);}
  function toggleVisualizer(){if(!visualizerOpen){visualizerOpener.current=document.activeElement as HTMLElement|null;setVisualizerView((preferences.visualizer??defaultVisualizer).lyrics?'lyrics':'playing');}setVisualizerOpen(v=>!v);}
  function chooseDestination(next:Destination){if(section==='Library'&&destination===next)return;window.dispatchEvent(new Event('musicos-navigation-start'));captureScroll();if(next==='search'&&destination!=='search')previousDestination.current=destination;setSection('Library');switchDestination(next);}
  function openSearch(){if(compact||route.view==='search'){chooseDestination('search');if(route.view!=='search')navigateScope('search');setSearchFocus(value=>value+1);setCommandOpen(false);}else setCommandOpen(value=>!value);}
  const mounted = useRef(true);
  const barMotion=useRef<HTMLElement>(null);usePlayerHandoff(barMotion,player.track?.id,"bar",player.queue_entry_ids?.[player.queue_index??-1]);
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
      if(gone)return;bootstrapSucceeded.current=!data.preference_error;savedSparkBeta.current=data.preferences.sparklist_beta;setPreferences(data.preferences);setPlayer(data.playback);setTask(data.task);
      if(data.preference_error)setSaveError(data.preference_error);
    });
    const libraryLoad=call("refresh_library").then(data=>{if(!gone){librarySucceeded.current=true;setLibrary(data);}});
    void Promise.all([lightweight,libraryLoad]).then(()=>{
      if(!gone)void call("collections",{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));
    }).catch(e=>{if(!gone)setError(errorText(e));}).finally(()=>{if(!gone)setLoading(false);});
    async function events() {
      const listeners = await Promise.all([
        subscribe<Partial<Playback>>("playback", (update) =>
          setPlayer(update),
        ),
        subscribe<Task>("task", task=>{
          if(task.kind==='enrich'&&!task.running)invalidateDisplay('album_metadata');
          setTask(task);
        }),
        subscribe<number[]>("lyrics-sidecars-updated",ids=>{for(const id of ids)window.dispatchEvent(new CustomEvent("musicos-lyrics-changed",{detail:id}));}),
        subscribe<LibraryChanged | null>("library-changed", (change) => {
          if(change?.reason!=="analysis"&&change?.reason!=="favorites")invalidateDisplay();
          window.dispatchEvent(new CustomEvent("musicos-library-changed",{detail:change}));
          if(change?.reason==='favorites'&&change.favorite_updates?.length){const updates=new Map(change.favorite_updates.map(update=>[update.track_id,update.favorited]));setLibrary(old=>({...old,tracks:old.tracks.map(track=>updates.has(track.id)?{...track,favorited:updates.get(track.id)!}:track)}));}
          else if(change?.reason==='collections')void call('collections',{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));
          else if(change?.reason!=="analysis")void refresh();
        }),
        subscribe("native-open-queue",()=>nativeQueueAction.current()),
        subscribe<string>("native-playback-error",setError),
        subscribe("audio-rescanned",()=>window.dispatchEvent(new Event("musicos-audio-rescanned"))),
        subscribe("artist-profile", (payload:any)=>{artistCompletions.current.set(payload.name,(artistCompletions.current.get(payload.name)??0)+1);setArtistLoading(old=>{const next=new Set(old);next.delete(payload.name);return next;});profileCache.set(payload.name,Promise.resolve(payload.info));rememberArtistInfo(payload.name,payload.info);setLibrary(old=>({...old,artists:old.artists.map(a=>a.name===payload.name?{...a,portrait:payload.info.portrait}:a)}));if(artistNameRef.current===payload.name)setArtistInfo(payload.info);}),
        subscribe<{track_id:number;path:string}>("apple-cover-cached",update=>{setLibrary(old=>({...old,tracks:old.tracks.map(t=>t.id===update.track_id&&!t.cover_art_path?{...t,cover_art_path:update.path}:t),albums:old.albums.map(a=>a.track_ids.includes(update.track_id)&&!a.cover?{...a,cover:update.path}:a)}));setPlayer(old=>old.track?.id===update.track_id&&!old.track.cover_art_path?{...old,track:{...old.track,cover_art_path:update.path}}:old);}),
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
              const {paths,position} = e.payload;
              if (!paths.length) return;
              const coverDrop=document.elementFromPoint(position.x/window.devicePixelRatio,position.y/window.devicePixelRatio)
                ?.closest('[data-album-cover-drop]');
              if(coverDrop){
                coverDrop.dispatchEvent(new CustomEvent('musicos-drop-cover',{detail:paths}));
                return;
              }
              void call("inspect_import", { paths })
                .then(async () => {
                  if(savingPreferences.current)await savePromise.current;
                  return call("library_task", {request:{kind:"copy",paths,root:dropTarget.current?.path,dropped:true}});
                })
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
  }, [route.view, route.id, section, destination, loading]);

  const album = library.albums.find((a) => a.id === route.id||a.legacy_ids?.includes(route.id??""));
  const artist = library.artists.find((a) => a.id === route.id||a.legacy_ids?.includes(route.id??""));
  const viewedArtistInfo=artist?profileInfoCache.get(artist.name)??null:null;
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
    let pending=false;
    try {const info=await request;if(started===(artistCompletions.current.get(name)??0))rememberArtistInfo(name,info);pending=info.lookup_status==='loading'&&started===(artistCompletions.current.get(name)??0);if(pending)setArtistLoading(old=>old.has(name)?old:new Set(old).add(name));
    if(info.portrait&&started===(artistCompletions.current.get(name)??0))setLibrary(old=>old.artists.some(a=>a.name===name&&a.portrait!==info.portrait)?({...old,artists:old.artists.map(a=>a.name===name?{...a,portrait:info.portrait}:a)}):old);
    return info;
    }finally{if(!pending)setArtistLoading(old=>{if(!old.has(name))return old;const next=new Set(old);next.delete(name);return next;});}
  },[]);
  useEffect(()=>{
    let gone=false;setArtistInfo(null);
    const names=route.view==='artist'&&artist?[artist.name]:route.view==='album'&&album?library.artists.filter(a=>a.album_ids.includes(album.id)).map(a=>a.name):[];
    void (async()=>{for(const name of names){if(gone)return;try{const info=await enrichArtist(name);if(!gone&&route.view==='artist')setArtistInfo(info);}catch(e){if(!gone&&route.view==='artist')setArtistInfo(null);}}})();
    return()=>{gone=true;};
  },[route.view,route.id,artist?.name,album?.id,enrichArtist]);
  useEffect(()=>{
    if(route.view!=='artists'||section!=='Library')return;
    let gone=false;
    const pending:string[]=[];let running=false;
    const pump=async()=>{if(running)return;running=true;while(pending.length&&!gone){try{await enrichArtist(pending.shift()!);}catch{}}running=false;};
    const observer=new IntersectionObserver(entries=>{for(const entry of entries){if(!entry.isIntersecting)continue;observer.unobserve(entry.target);const name=entry.target.getAttribute('data-artist-name');if(name&&!gone&&!pending.includes(name)){pending.push(name);}}void pump();},{root:scrollRef.current,rootMargin:'100px'});
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
  const playingCover = playingAlbum?.cover ??
    library.tracks.find(t=>t.id===player.track?.id)?.cover_art_path ??
    player.track?.cover_art_path;
  const [playingArtPalette,setPlayingArtPalette]=useState<Palette|null>(null);
  const playingThemeRequest=JSON.stringify([playingCover??null,playingAlbum?.track_ids[0]??player.track?.id]);
  useEffect(()=>{
    let gone=false;
    if(loading||look.source!=='artwork'){setPlayingArtPalette(null);return;}
    const [cover,id]=JSON.parse(playingThemeRequest) as [string|null,number|undefined];
    void resolveArtworkTheme([cover],id).then(result=>{if(!gone&&(result||id===undefined))setPlayingArtPalette(result?.palette??null);});
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
    route.view === "catalog" ? route.catalog?.cover : album?.cover ?? (album?.id===playingAlbum?.id ? playingCover : null),
    viewedArtistInfo?.portrait ?? artist?.portrait,
    firstArtistAlbum?.cover,
    playingCover,
  );
  const artistThemeTracks=useMemo(()=>library.albums.filter(a=>artist?.album_ids.includes(a.id)).map(a=>a.track_ids[0]).filter((id):id is number=>id!==undefined),[library.albums,artist?.id]);
  const themeTrackId = section === "Library" && route.view === "album"
    ? album?.track_ids
    : section === "Library" && route.view === "artist" ? artistThemeTracks : playingAlbum?.track_ids[0]??player.track?.id;
  const themeRequest=JSON.stringify([[source,section==='Library'&&route.view==='artist'?firstArtistAlbum?.cover:null],themeTrackId]);
  useEffect(() => {
    let gone=false;
    async function resolve() {
      if(loading)return;
      const [paths,tracks]=JSON.parse(themeRequest) as [(string|null)[],number|number[]|undefined];
      const result=look.source==='artwork'?await resolveArtworkTheme(paths,tracks??undefined):null;
      if(!gone)setResolvedArtwork(previous=>{
        // Preserve colors if the same artwork is briefly unavailable during a refresh.
        if(!result&&look.source==='artwork'&&previous?.result)return {...previous,request:themeRequest};
        return {request:themeRequest,result};
      });
    }
    void resolve();return ()=>{gone=true;};
  }, [loading,look.source,themeRequest,themeEpoch]);
  const inputModality=useRef<ReturnType<typeof observeInputModality>|null>(null);
  useEffect(()=>{const observer=observeInputModality();inputModality.current=observer;return()=>{observer.dispose();inputModality.current=null;};},[]);
  const nativeQueueAction=useRef<()=>void>(()=>{});
  const run = useCallback(async (action: PlaybackAction) => {
    if(action.type==='next'||action.type==='previous')window.dispatchEvent(new CustomEvent('musicos-transport-intent',{detail:action.type}));
    try {
      setPlayer(await call("playback", { action }));
      if(action.type==='seek')window.dispatchEvent(new Event('musicos-playback-discontinuity'));
    } catch (e) {
      setError(errorText(e));
    }
  }, []);
  const runImmersive=useCallback(async(action:PlaybackAction)=>{if(action.type==='next'||action.type==='previous')window.dispatchEvent(new CustomEvent('musicos-transport-intent',{detail:action.type}));setPlayer(await call('playback',{action}));if(action.type==='seek')window.dispatchEvent(new Event('musicos-playback-discontinuity'));},[]);
  const shortcutDispatch=useRef<(shortcut:PlayerShortcut)=>void>(()=>{});
  shortcutDispatch.current=shortcut=>{if(document.querySelector(".onboarding[open]"))return;if(shortcut==='previous'||shortcut==='next')void run({type:shortcut});else if(shortcut==='visualizer')toggleVisualizer();else if(visualizerOpen){const next=nextPlayerView(visualizerView);setVisualizerView(next);changePreferences({...preferences,visualizer:{...(preferences.visualizer??defaultVisualizer),lyrics:next==='lyrics'}});}else if(expandedOpen)setPlayerView(nextPlayerView(playerView));else if(miniMode!=='normal')window.dispatchEvent(new Event('musicos-cycle-mini-view'));else{const next=nextPlayerView(queueOpen?queueView:'playing');if(next!=='playing')setQueueView(next);setQueueOpen(next!=='playing');}};
  const controllerActions=useRef({play:()=>{},previous:()=>{},next:()=>{},panel:()=>{},visualizer:()=>{},favorite:()=>{},seek:(_delta:number)=>{},back:()=>{},error:(e:unknown)=>setError(errorText(e))});
  controllerActions.current={play:()=>{if(player.track)void run({type:'toggle'});},previous:()=>{if(player.track)void run({type:'previous'});},next:()=>{if(player.track)void run({type:'next'});},panel:()=>shortcutDispatch.current('panel'),visualizer:()=>shortcutDispatch.current('visualizer'),favorite:()=>{if(player.track)void favorite(player.track);},seek:delta=>{if(player.track)void run({type:'seek',position:Math.max(0,Math.min(player.track.duration,clockPosition(currentPlaybackClock())+delta))});},back:()=>{if(queueOpen)setQueueOpen(false);else if(navigation.history.length)goBack();},error:e=>setError(errorText(e))};
  useEffect(()=>installController(preferences.controller??defaultController,Object.fromEntries(Object.keys(controllerActions.current).map(key=>[key,(...args:unknown[])=>(controllerActions.current[key as keyof typeof controllerActions.current] as (...v:unknown[])=>void)(...args)])) as typeof controllerActions.current),[preferences.controller]);
  useNativePlayerShortcuts(shortcut=>shortcutDispatch.current(shortcut));
  useEffect(() => {
    const pointerSpace=(e:KeyboardEvent)=>{
      const target=e.target as HTMLElement;
      if(e.code!=="Space"||e.defaultPrevented||e.metaKey||e.ctrlKey||e.altKey||!player.track||!inputModality.current?.pointerFocus())return;
      if(!target.closest("button,[role=button]")||target.closest("dialog,[role=menu],input,textarea,select,[contenteditable=true]")||document.querySelector('[role=menu]:not([inert])'))return;
      e.preventDefault();e.stopPropagation();if(!e.repeat)void run({type:"toggle"});
    };
    window.addEventListener("keydown",pointerSpace,true);
    const listener = (e: KeyboardEvent) => {
      if (e.defaultPrevented||document.querySelector(".onboarding[open]")) return;
      const target = e.target as HTMLElement;
      const shortcut=playerShortcut(e);
      if(shortcut){e.preventDefault();e.stopPropagation();shortcutDispatch.current(shortcut);return;}

      if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){
        e.preventDefault();if(!document.querySelector('dialog[open]:not(.supercommand)'))openSearch();return;
      }
      if((e.metaKey||e.ctrlKey)&&!e.altKey&&e.key.toLowerCase()==='f'&&section==='Library'&&miniMode==='normal'&&(findSupported||collectionFind)&&!commandOpen&&!document.querySelector('dialog[open],[role=menu]')){
        e.preventDefault();openFind();return;
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
    return () => {window.removeEventListener("keydown", listener);window.removeEventListener("keydown",pointerSpace,true);};
  }, [run, section, queueOpen, queueView, expandedOpen, playerView, visualizerOpen,visualizerView,preferences.visualizer,player.track?.id,findRoute,findOpen,miniMode,commandOpen,compact,destination,route.view]);
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
    if(savingPreferences.current)return savePromise.current;
    savingPreferences.current=true;
    savePromise.current=(async()=>{
      let success=true;
      try {while(savePending.current){const latest=savePending.current;savePending.current=null;try {await call("save_preferences",{preferences:latest});if(savedSparkBeta.current!==latest.sparklist_beta){savedSparkBeta.current=latest.sparklist_beta;invalidateDisplay('sparklist');}if(mounted.current)setSaveError("");}catch(e){success=false;if(mounted.current)setSaveError(`Could not save settings. ${errorText(e)}`);}}}
      finally {savingPreferences.current=false;}
      return success;
    })();
    return savePromise.current;
  }
  async function setupImport(){
    if(!native)throw Error('Open the desktop app to choose music folders. You can still explore first.');
    const selected=await open({directory:true,multiple:true,title:'Choose music folders'});
    if(!selected)return [];const paths=typeof selected==='string'?[selected]:selected;
    await call('library_task',{request:{kind:'reference',paths}});return paths;
  }
  async function finishSetup(next:Preferences,devices:boolean){
    const saved=await changePreferences(next);if(!saved)return false;
    setOnboardingOpen(false);if(devices)setSection('Devices');return true;
  }
  function captureScroll() {
    if(section==='Settings'&&scrollRef.current)settingsScroll.current.set(settingsKey.current,scrollRef.current.scrollTop);
    if (section === "Library" && scrollRef.current)
      dispatch({
        type: "patch",
        patch: { scroll: scrollRef.current.scrollTop },
      });
  }
  function goBack(interactive=false,artwork?:BackArtwork) {
    if(artwork)setResolvedArtwork(artwork);
    if(!interactive&&section==='Library'&&route.view==='album')captureAlbumReturn(scrollRef.current);
    else if(!interactive&&section==='Library'&&route.view==='artist')captureArtistReturn(scrollRef.current);
    window.dispatchEvent(new Event("musicos-navigation-start"));
    captureScroll();
    dispatch({ type: "back" });
  }
  function chooseSection(next: "Library" | "Devices" | "Settings") {
    window.dispatchEvent(new Event("musicos-navigation-start"));
    captureScroll();
    setSection(next);
  }
  function openRoute(next: Route) {
    const artwork=resolvedArtwork?.request===themeRequest?resolvedArtwork:undefined;
    captureBackPage(scrollRef.current,JSON.stringify({...route,scroll:0}),swipeAppearanceKey,artwork);
    window.dispatchEvent(new Event("musicos-navigation-start"));
    if(section==='Library'&&['album','artist'].includes(next.view))captureLibraryToolbar(scrollRef.current);
    captureScroll();
    dispatch({ type: "open", route: next });
  }
  function goScope(view: Route["view"]) {
    window.dispatchEvent(new Event("musicos-navigation-start"));
    captureScroll();
    navigateScope(view);
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
        throw Error("Open the musicOS desktop app to add your music.");
      const result = await open({
        directory: true,
        multiple: true,
        title: copy ? "Copy music into musicOS" : "Add a music folder",
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
  function startTask(kind: string, root?: string, albumId?:string) {
    void call("library_task", { request: { kind, root, album_id:albumId } }).catch((e) =>
      setError(errorText(e)),
    );
  }
  const needle=route.search.trim();
  const findMatch=useMemo(()=>compileListFind(needle),[needle]);
  const scopedTracks=useMemo(()=>{
    let records=library.tracks;
    if(route.view==='album')records=(album?.track_ids??[]).map(id=>tracksById.get(id)!).filter(Boolean);
    if(route.view==='artist')records=records.filter(track=>artist?.track_ids.includes(track.id));
    if(route.view==='playlist')records=(collections.find(item=>item.id===route.id)?.track_ids??[]).map(id=>tracksById.get(id)!).filter(Boolean);
    if(route.view==='favorites')records=records.filter(track=>track.favorited);
    return records;
  },[library,route.view,route.id,collections,album,artist,tracksById]);
  const tracks=useMemo(()=>scopedTracks.filter(findMatch),[scopedTracks,findMatch]);
  const genreScope=route.view==='genre'?library.genres.find(genre=>genre.id===route.id):null;
  const albumFilters={...defaultFilters,...preferences.library_filters};
  const scopedAlbums=useMemo(()=>['albums','genre'].includes(route.view)?filterAlbums(library,albumFilters).filter(a=>!genreScope||genreScope.album_ids.includes(a.id)):[],[library,preferences.library_filters,route.view,genreScope]);
  const foundAlbums=useMemo(()=>scopedAlbums.filter(a=>findMatch({title:a.title,album:a.title,artist:a.artist,genre:a.genre,year:a.year,favorited:a.track_ids.some(id=>tracksById.get(id)?.favorited),available:a.track_ids.some(id=>tracksById.get(id)?.available)})),[scopedAlbums,findMatch,tracksById]);
  const foundGenres=useMemo(()=>library.genres.filter(genre=>findMatch({title:genre.name,genre:genre.name})),[library.genres,findMatch]);
  const findGrid=['albums','genre'].includes(route.view),findGenres=route.view==='genres';
  const findCount=findGrid?foundAlbums.length:findGenres?foundGenres.length:tracks.length;
  const findTotal=findGrid?scopedAlbums.length:findGenres?library.genres.length:scopedTracks.length;
  const findUnit=findGrid?'album':findGenres?'genre':'song';
  const findContext=route.view==='playlist'?(collections.find(item=>item.id===route.id)?.name??'this playlist'):route.view==='genre'?(genreScope?.name??'this genre'):route.view==='favorites'?'Favorites':route.view==='songs'?'Songs':route.view==='albums'?'Albums':'Genres';
  async function queueTracks(ids:number[],next=false){const playable=ids.filter(id=>availableIds.has(id));for(const id of next?[...playable].reverse():playable)await run({type:next?'play_next':'add_to_queue',id});}
  function playbackItems(ids:number[]){return [
   {label:'Play',icon:'play',disabled:!ids.some(id=>availableIds.has(id)),onSelect:()=>playIds(ids)},
   {label:'Shuffle',icon:'ShufflePlay',disabled:!ids.some(id=>availableIds.has(id)),onSelect:()=>playIds(ids,true)},
   {label:'Play next',icon:'Play Next',disabled:!ids.some(id=>availableIds.has(id)),onSelect:()=>void queueTracks(ids,true)},
   {label:'Add to queue',icon:'Add to queue',disabled:!ids.some(id=>availableIds.has(id)),onSelect:()=>void queueTracks(ids)},
  ];}
  function actionsForTrack(track:Track,ids=[track.id]){return [...playbackItems(ids),
   {label:track.favorited?'Unfavorite song':'Favorite song',icon:track.favorited?'hearted':'noheart',onSelect:()=>favorite(track)},
   {label:'Add to playlist…',icon:'Playlist',onSelect:()=>setPlaylistTrack({...track,selectionIds:ids})},
   {label:'Visualizer',icon:'Visualizer',onSelect:()=>toggleVisualizer()},
    {label:'Now playing',icon:'Song',disabled:!player.track,onSelect:()=>openPlayer('playing')},
    {label:'Lyrics',icon:'Lyrics',onSelect:()=>openPlayer('lyrics',track.id===player.track?.id?null:track)},
    {label:'Edit lyrics',icon:'Lyrics',onSelect:()=>openMetadata({mode:'lyrics',trackId:track.id})},
   {label:'Show album',icon:'Album',onSelect:()=>albumFor(track)},
   {label:native?'Show in Finder':'Show in file manager',icon:native?'Open In Finder':'Open In Folder',disabled:!track.available,onSelect:()=>void call('reveal_track',{trackId:track.id}).catch(e=>setError(errorText(e)))},
   {label:'Show artist',icon:'Artist',onSelect:()=>artistFor(track)},
  ];}
  function actionsForAlbum(a:Album){return [...playbackItems(a.track_ids),...[
        {label:"Add to playlist…",icon:"Playlist",onSelect:()=>setPlaylistTrack({...library.tracks.find(t=>t.id===a.track_ids[0])!,selectionIds:a.track_ids})},
        {label:"Edit metadata",icon:"Metadata",onSelect:()=>openMetadata({mode:'album',albumId:a.id})},
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
      ].filter(item=>!['Play','Shuffle'].includes(item.label))];}
  function actionsForCollection(item:SavedCollection){return [...playbackItems(item.track_ids),
   {label:'Open collection',icon:'Playlist',onSelect:()=>openCollection(item)},
   {label:item.spark?'Edit rules…':'Edit details…',icon:'File',onSelect:()=>{openCollection(item);setCollectionEdit({id:item.id,revision:Date.now()});}},
   ...(item.kind!=='audio'&&!item.spark?[{label:'Export M3U…',icon:'File',onSelect:()=>void playlistFile('export',item.id).catch(e=>setError(errorText(e)))}]:[]),
   {label:'Delete collection…',icon:'Delete',onSelect:()=>setDeletingCollection(item)},
  ];}
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
      menu: actionsForAlbum(a),
    }));
  }
  const selected = (id: string) =>
    dispatch({ type: "patch", patch: { selected: id } });
  const rows = (records: Track[]) => {const content=(
    <TrackRows
      tracks={records}
      playInIndex={route.view==='favorites'}
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
      onEdit={(track,ids)=>openMetadata(ids&&ids.length>1?{mode:'bulk',ids}:{mode:'song',trackId:ids?.[0]??track.id})}
      onFavorite={track=>void favorite(track)}
      playlistActions={actionsForTrack}
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
  const collectionActions = (ids: number[], artistAction?: () => void, editAction?: () => void) => (
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
          ...(editAction ? [{label:"Edit metadata",icon:"Metadata",onSelect:editAction}] : []),
        ]}
      />
    </div>
  );
  const sidebarScroll = useRef<HTMLDivElement>(null);
  useScrollEdges(sidebarScroll, section==='Library'&&!compact, collections);
  let content;
  if (miniMode!=='normal')content=null;
  else if (loading)
    content = (
      <div className="loading" role="status">

      </div>
    );
  else if(route.view==='search')content=null;
  else if(route.view==='home')content=<PersonalHome name={preferences.display_name} library={library} collections={collections} recommendations={sparkRecommendations} player={player} canResume={!homePlaybackStarted&&!player.is_playing} active={section==='Library'&&miniMode==='normal'} onAlbum={openAlbum} betaEnabled={!!preferences.sparklist_beta} onMixIdea={seed=>{setSparkSeed(seed);setCreatingCollection('spark');}} trackActions={actionsForTrack} albumActions={actionsForAlbum} collectionActions={actionsForCollection} onCollection={openCollection} onPlay={ids=>playIds(ids)} onToggle={()=>void run({type:'toggle'})} onScope={goScope} onCreate={createCollection} onAdd={()=>void chooseFolder(false)}/>;
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
            <div className="album-footer"><AlbumDescription key={`description-${album.id}`} albumId={album.id}/><div className="album-footer-controls"><AudioQuality key={album.id} tracks={tracks}/>
            {collectionActions(album.track_ids, () => {
              const a = library.artists.find((a) => a.id === album.artist_id);
              if (a) openArtist(a);
            }, () => openMetadata({mode:"album",albumId:album.id}))}</div></div>
          </div>
        </header>
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
        <ArtistOverview key={artist.id} artist={artist} albums={albums} tracks={tracks} info={viewedArtistInfo}
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
        copyOnlyNonLibraryDrops={!!preferences.copy_only_non_library_drops}
        onCopyOnlyNonLibraryDropsChange={copy_only_non_library_drops=>changePreferences({...preferences,copy_only_non_library_drops})}
        busy={task.running}
        task={task}
        playing={player.is_playing}
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
  else if(route.view==='sparklists'||route.view==='playlists') content=<CollectionManager findRequest={findKey===findRoute?findRequest:0} library={library} items={collections} onChange={setCollections} onOpen={openCollection} actionsFor={actionsForCollection} initialTab={route.view==='sparklists'?'audio':'manual'} onCreate={createCollection} onSetup={()=>{sessionStorage.setItem('musicos-audio-page','spark');sessionStorage.setItem('musicos-settings-tab','Audio');setSection('Settings');}}/>;
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
        Bring your albums, artists, and favorite songs into musicOS.
      </Empty>
    );
  else if(route.view==='playlist') {
    const item=collections.find(c=>c.id===route.id);
    const Detail=item?.kind==='audio'?AudioSparkDetail:PlaylistDetail;
    content=item?<Detail finding={!!needle} audioModel={automaticModel.state} key={item.id} editRequest={collectionEdit?.id===item.id?collectionEdit.revision:0} item={item} items={collections} library={library} onChange={setCollections} onDeleted={()=>navigation.history.length?goBack():goScope('albums')} scale={scale} visibleTrackIds={tracks.map(t=>t.id)} actions={collectionActions(tracks.map(t=>t.id))}>{needle&&!tracks.length?<p className="find-empty" role="status">No matching songs. Try another term or clear Find.</p>:rows(tracks)}</Detail>:<Empty title="Playlist unavailable">It may have been removed.</Empty>;
  }
  else if (route.view === "songs" || route.view === "favorites")
    content = tracks.length ? (
      <>{route.view==='favorites'&&<div className="favorites-play-actions">{collectionActions(tracks.map(track=>track.id))}</div>}{rows(tracks)}</>
    ) : (
      <Empty title="No songs found.">
        Try another search or add more music.
      </Empty>
    );
  else if (route.view === "artists")
    content = grid(
      (preferences.artists_album_only?albumArtists(library):library.artists)
        .filter((a) => findMatch({title:a.name,artist:a.name}))
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
      foundGenres
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
    const filters=albumFilters;
    const albums=[...foundAlbums].sort((a,b)=>filters.sort==='title'?a.title.localeCompare(b.title):b.track_ids.reduce((m,id)=>Math.max(m,id),0)-a.track_ids.reduce((m,id)=>Math.max(m,id),0));
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
  const miniOperation=useRef(0),cancelMiniExit=useRef<(()=>void)|undefined>(undefined);
  useEffect(()=>{if(!native||miniMode==='normal')return;let alive=true,off:(()=>void)|undefined,closing=false;void import('@tauri-apps/api/window').then(async({getCurrentWindow})=>{const w=getCurrentWindow();const stop=await w.onCloseRequested(async event=>{if(closing)return;event.preventDefault();closing=true;const node=document.querySelector<HTMLElement>('.mini-player');if(node){node.inert=true;animateOwned(node,'mini-native-close',[{opacity:1,transform:'none'},{opacity:0,transform:'translateY(4px)'}],140);await Promise.allSettled(node.getAnimations().map(a=>a.finished));}if(alive)await w.close();});if(alive)off=stop;else stop();});return()=>{alive=false;off?.();};},[miniMode]);
  async function changeMini(mode:'normal'|'tall'|'queue') {
    const operation=++miniOperation.current;cancelMiniExit.current?.();cancelMiniExit.current=undefined;
    if(miniChanging.current){miniChanging.current=false;if(mode===miniMode)return;}
    miniChanging.current=true;
    let exit:(()=>void)|undefined;let exitNode:HTMLElement|null=null;
    try {if(mode==='normal'&&miniMode!=='normal'){const node=document.querySelector<HTMLElement>('.mini-shell>.mini-player');if(node){exitNode=node;node.inert=true;}if(node&&!matchMedia('(prefers-reduced-motion: reduce)').matches){exit=animateOwned(node,'mini-window-exit',[{opacity:getComputedStyle(node).opacity,transform:getComputedStyle(node).transform},{opacity:0,transform:'translateY(4px)'}],140);cancelMiniExit.current=exit;await Promise.allSettled(node.getAnimations().map(animation=>animation.finished));}}if(operation!==miniOperation.current)return;if(native)await call('mini_player',{mode,pinned:miniPinned});setMiniMode(mode);setQueueOpen(false);if(mode!=='normal')changePreferences({...preferences,mini_player_view:mode==='queue'?'queue':'artwork'});}
    catch(e){if(native)await call('mini_player',{mode:'normal'}).catch(()=>{});setMiniMode('normal');setError(errorText(e));}
    finally {exit?.();if(exitNode?.isConnected)exitNode.inert=false;if(operation===miniOperation.current){miniChanging.current=false;cancelMiniExit.current=undefined;}}
  }
  async function toggleWindowPin(){
    const enabled=!(miniMode==='normal'?windowPinned:miniPinned);
    try{if(native)await call('window_pin',{enabled});if(miniMode==='normal')setWindowPinned(enabled);else setMiniPinned(enabled);}
    catch(e){setError(errorText(e));}
  }
  async function showPlaying(kind:'artist'|'album',name?:string) {
    if(miniMode!=='normal')await changeMini('normal');
    if(player.track){setExpandedOpen(false);setSection('Library');if(kind==='artist')artistFor(player.track,name);else albumFor(player.track);}
  }
  async function showQueue(){if(miniMode!=='normal')await changeMini('normal');if(compact||expandedOpen)openPlayer('queue');else {setQueueView('queue');setQueueOpen(true);}}
  nativeQueueAction.current=()=>void showQueue();
  const nowActions = player.track ? [
    {label:'Lyrics',icon:'Lyrics',onSelect:()=>{void (async()=>{if(miniMode!=='normal')await changeMini('normal');openPlayer('lyrics');})();}},
    {label:player.track.favorited?'Unfavorite song':'Favorite song',icon:player.track.favorited?'hearted':'noheart',onSelect:()=>favorite(player.track!)},
    {label:'Add to playlist…',icon:'Playlist',onSelect:()=>{void (async()=>{if(miniMode!=='normal')await changeMini('normal');setExpandedOpen(false);setPlaylistTrack(player.track);})();}},
    {label:'Show artist',icon:'Artist',onSelect:()=>void showPlaying('artist')},
    {label:'Show album',icon:'Album',onSelect:()=>void showPlaying('album')},
    {label:'Show in Finder',icon:'Open In Finder',disabled:!player.track.available,onSelect:()=>void call('reveal_track',{trackId:player.track!.id}).catch(e=>setError(errorText(e)))},
    {label:'Edit song details',icon:'Song',onSelect:()=>{void (async()=>{if(miniMode!=='normal')await changeMini('normal');setExpandedOpen(false);openMetadata({mode:'song',trackId:player.track!.id});})();}},
  ] : [];

  const displayScope = (
    ["album", "artist", "genre"].includes(route.view)
      ? `${route.view}s`
      : route.view
  ) as Scope;
  return (<SongPreviewProvider active={miniMode==='normal'&&!visualizerOpen}><><ViewActivity active={!visualizerOpen}>
    {miniMode!=='normal'&&(<div className="app-shell mini-shell" inert={visualizerOpen} style={{...playingStyle,background:"transparent"}} data-theme-mode={look.mode==='system'?(systemDark?'dark':'light'):look.mode}><MiniPlayer onViewChange={view=>{if(view!=='lyrics')changePreferences({...preferences,mini_player_view:view==='queue'?'queue':'artwork'});}} density={preferences.mini_player_density??"standard"} player={player} cover={playingCover} bpm={player.track?tempos.get(player.track.id):undefined} mode={miniMode} onMode={mode=>void changeMini(mode)} onAction={run} onFavorite={()=>{if(player.track)favorite(player.track);}} onAlbum={()=>void showPlaying('album')} onQueue={()=>void changeMini(miniMode==='queue'?'tall':'queue')} onMenuResize={async open=>{if(native)await call('mini_player',{mode:miniMode,menuOpen:open,pinned:miniPinned});}} pinned={miniPinned} onPin={()=>void toggleWindowPin()} onWindowError={setError} items={nowActions}/>{error&&<button className="mini-player-error" onClick={()=>{void changeMini('normal');}}>{error} · Return to musicOS</button>}</div>)}
    <KeepView className="main-view" active={miniMode==='normal'}>{miniMode==='normal'&&<div
      className="app-shell adaptive-shell" inert={visualizerOpen} ref={shell.root} data-layout={shell.layout} data-audio-sidebar={dockedAudio||undefined}
      style={{'--playing-favorite-tone':playingPalette?.accent??'var(--accent)'} as CSSProperties}
      data-size={preferences.interface_percent}
      data-adaptive={!!palette}
      data-theme-mode={look.mode==='system'?(systemDark?'dark':'light'):look.mode}
    >
      <div ref={backSwipeCue} className="back-swipe-cue" aria-hidden="true"><Icon name="Back"/></div>
      <a className="skip-link" href="#library-content">
        Skip to content
      </a>
      <header className="app-header" onMouseDown={e=>dragWindow(e,setError)}>
        <div
          className="window-drag-region"
        ><div className="header-brand"><Icon name="Album"/><span>{product.name}</span></div></div>
        <div className="section-control">
        <Menu
          label="Choose section"
          selectedLabel={section}
          onIntent={()=>{void import("./Settings");void import("./Devices");void import("./CollectionManager");}}
          items={(["Library", "Devices", "Settings"] as const).map((label) => ({
            label,
            icon: label==='Settings'?'Settings':label==='Devices'?'device':'Library',
            onSelect: () => chooseSection(label),
          }))}
        >
          <span className="section-icon-space" aria-hidden="true"/><span className="section-name">{section}</span>
          <span className="chevron" aria-hidden="true"><svg viewBox="0 0 24 12"><path d="M1 1 12 11 23 1" /></svg></span>
        </Menu>
        <Menu triggerClassName="section-actions-trigger" label="musicOS actions" items={[{label:'Activity',icon:'sync',onSelect:()=>setActivityOpen(true)},{label:'Mini Player',icon:'openMiniplayer',onSelect:()=>void changeMini(preferences.mini_player_view==='queue'?'queue':'tall')},{label:'Manage collections',icon:'Playlist',onSelect:()=>{setSection('Library');goScope('playlists');}}]}><span className="section-action-glyph"><Icon key={section} name={section==='Library'?'Library':section==='Settings'?'Settings':'device'} className="section-category-glyph"/><Icon name="3dots_(nocircle)" className="section-dots-glyph"/></span></Menu>
        </div>
        <div className="header-drag-spacer" />
        <section ref={barMotion} style={playingStyle} className="now-playing" aria-label="Now playing" onContextMenu={e=>{e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('.context-only')?.dispatchEvent(new CustomEvent('musicos-open-menu',{detail:{x:e.clientX,y:e.clientY}}));}} onKeyDown={e=>{if(e.key==='ContextMenu'||(e.shiftKey&&e.key==='F10')){e.preventDefault();e.currentTarget.querySelector<HTMLButtonElement>('.context-only')?.dispatchEvent(new Event('musicos-open-menu'));}}}>
          <Overflow theme={playingStyle} contextOnly items={[{label:'Mini Player',icon:'openMiniplayer',onSelect:()=>void changeMini(preferences.mini_player_view==='queue'?'queue':'tall')},{label:windowPinned?'Unpin window':'Pin window',icon:windowPinned?'unPin':'Pin',onSelect:()=>void toggleWindowPin()},...nowActions]}/>
          <button
            className="now-playing-cover"
            aria-label={compact?"Open player":preferences.now_playing_artwork_action==='artwork'?"Show playing artwork":preferences.now_playing_artwork_action==='mini_player'?"Open Mini Player":"Show playing album"}
            onClick={event => {
              if(compact){openPlayer();return;}
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
              <button className="now-playing-title-action" disabled={!player.track} aria-label="Open lyrics" onClick={()=>openPlayer('lyrics')}><OverflowLabel text={player.track?.title??(loading?'':'Nothing playing yet')}/></button>
              {player.track&&<FavoriteButton track={player.track} bpm={tempos.get(player.track.id)} playing={!expandedPresent&&player.is_playing} onFavorite={favorite} className="now-playing-heart" label={player.track.favorited?'Unfavorite playing song':'Favorite playing song'}/>}
            </strong>
            <span className="now-playing-credits">{player.track?<><span className="now-playing-artists">{trackCredits(player.track).map((name,index)=><span key={name}>{index>0&&<i aria-hidden="true"> + </i>}<button onClick={()=>void showPlaying('artist',name)} title={name}>{name}</button></span>)}</span><i aria-hidden="true"> · </i><button className="now-playing-album" onClick={()=>void showPlaying('album')} title={player.track.album}><OverflowLabel text={player.track.album}/></button></>:""}</span>
            <PlaybackProgress djMode={!!player.dj_mode} revealTime showTime={!loading&&preferences.show_now_playing_time!==false} transition={player.dj_transition} plannedMix={queuedMixForCurrent(player)} trackId={player.track?.id} entryId={player.queue_entry_ids?.[player.queue_index??-1]} position={player.position}
              duration={player.track?.duration ?? 0}
              seek={async position => { setPlayer(await call("playback", {action:{type:"seek",position}}));window.dispatchEvent(new Event("musicos-playback-discontinuity")); }} />
          </div>
          <div className="mini-controls">
            <PlayerViewCycle value={queueOpen?queueView:'playing'} onChange={next=>{if(compact){openPlayer(next);}else{if(next!=='playing')setQueueView(next);setQueueOpen(next!=='playing');}}}/>
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
      {compact&&<nav className="bottom-navigation" aria-label="Main navigation">{(['home','search','library'] as Destination[]).map(tab=><button data-playful key={tab} aria-current={section==='Library'&&destination===tab?'page':undefined} onClick={()=>chooseDestination(tab)}><Icon name={tab==='home'?'Home':tab==='search'?'search':'Library'}/><span>{tab[0].toUpperCase()+tab.slice(1)}</span></button>)}</nav>}
      <div className={`workspace ${section !== "Library" ? "full-width" : ""}`}>
        {section === "Library" && !compact && (
          <aside className="library-sidebar">
            <div className="sidebar-body scroll-edges" ref={sidebarScroll}>
            <div className="sidebar-body-content">
            <section className="sidebar-personal" aria-label="Your home"><button className={`sidebar-home ${route.view==='home'?'active':''}`} aria-current={route.view==='home'?'page':undefined} onClick={()=>goScope('home')}><Icon name="Home"/><span>{personalName(preferences.display_name,!loading)}</span></button><Menu label="Create collection" morph="plus" items={[{label:'New playlist',icon:'Playlist',onSelect:()=>createCollection('manual')},{label:'New Sparklist',icon:'Sparklist',onSelect:()=>createCollection('spark')},{label:'New Smart playlist',icon:'SmartPlaylist',onSelect:()=>createCollection('rule')}]}>+</Menu></section>
            <div className="sidebar-search-slot" data-back={navigation.history.length>0}>
              <button className={`sidebar-navigation ${navigation.history.length?'sidebar-back':'search sidebar-search'}`} aria-label={navigation.history.length?`Back to ${navigation.history.at(-1)!.view==='home'?'Home':navigation.history.at(-1)!.view[0].toUpperCase()+navigation.history.at(-1)!.view.slice(1)}`:'Search music and commands'} aria-keyshortcuts={navigation.history.length?undefined:'Meta+K Control+K'} onClick={navigation.history.length?()=>goBack():()=>setCommandOpen(true)}><span className="sidebar-nav-glyph" aria-hidden="true"><span className="sidebar-glyph-search"><Icon name="search"/></span><span className="sidebar-glyph-back"><Icon name="Back"/></span></span><span className="sidebar-nav-label">{navigation.history.length?`Back to ${navigation.history.at(-1)!.view==='home'?'Home':navigation.history.at(-1)!.view[0].toUpperCase()+navigation.history.at(-1)!.view.slice(1)}`:'Search'}</span>{!navigation.history.length&&<kbd>⌘ K</kbd>}</button>
              <button className="command-shortcut" aria-label="Open SuperCommand" aria-hidden={!navigation.history.length} inert={!navigation.history.length} tabIndex={navigation.history.length?0:-1} title="SuperCommand (⌘K / Ctrl+K)" onClick={()=>setCommandOpen(true)}><Icon name="search"/></button>
            </div>
            <nav aria-label="Library">
              {(["albums", "artists", "songs"] as Scope[]).map(
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
            <div className="sidebar-more"><Menu label="More library views" morph="dots" items={[{label:'Genres',icon:'Genre',onSelect:()=>goScope('genres')},{label:'Playlists',icon:'Playlist',onSelect:()=>goScope('playlists')},{label:'Sparklists',icon:'Sparklist',onSelect:()=>goScope('sparklists')},]}><Icon name="3dots_(nocircle)"/><span>More</span></Menu></div>
            <section className="sidebar-playlists" aria-label="Your playlists">
              <h2>{personalName(preferences.display_name,!loading)}</h2>
              <div className="sidebar-playlist-list">
                <button className="sidebar-favorites" aria-current={route.view==='favorites'?'page':undefined} onClick={()=>goScope('favorites')}>
                  <Icon name="hearted"/><span>Favorites</span>
                </button>
                {collections.filter(item=>item.kind!=='audio'&&!item.spark).map(item=>{
                  const summary=collectionSummaries.get(item.id)!;
                  return <ActionSurface contextOnly items={actionsForCollection(item)} key={item.id} className={`personal-collection ${route.view==='playlist'&&route.id===item.id?'active':''}`} data-collection-tone="true" data-manual-collection={item.kind!=='audio'&&!item.spark?item.id:undefined}>
                    <button className="collection-open" aria-label={`Open playlist ${item.name}`} aria-current={route.view==='playlist'&&route.id===item.id?'page':undefined} onClick={()=>openCollection(item)}><PlaylistArtwork item={item} library={library}/><span className="collection-label"><strong>{item.name}</strong><small>{item.track_ids.length} {item.track_ids.length===1?'song':'songs'}</small></span></button>
                    <button className="collection-shuffle" aria-label={`Shuffle ${item.name}`} disabled={!summary.available} onClick={()=>playIds(item.track_ids,true)}><Icon name="CollectionShuffle"/></button>
                  </ActionSurface>;
                })}
                {!collections.some(item=>item.kind!=='audio'&&!item.spark)&&<p>Your saved playlists appear here.</p>}
              </div>
            </section>
            {collectionError&&<small role="alert">{collectionError}</small>}
            </div>
            </div>
            <footer className="sidebar-footer">
              <span aria-busy={loading}>{loading?"":`${library.tracks.length} songs`}</span>
              <span>
                {loading?"":collectionDuration(librarySummary.duration)}
              </span>
            </footer>
          </aside>
        )}
        <div className="content-column">
          {compact&&section==='Library'&&<div className="compact-library-controls">
            {navigation.history.length>0&&<IconButton name="Back" label="Back" onClick={()=>goBack()}/>}
            {destination==='library'?<nav aria-label="Library categories">{(['albums','artists','songs'] as Scope[]).map(view=><button key={view} aria-current={displayScope===view?'page':undefined} onClick={()=>goScope(view)}><Icon name={scopeIcons[view]}/><span>{view[0].toUpperCase()+view.slice(1)}</span></button>)}<Menu label="More library views" morph="dots" items={[{label:'Genres',icon:'Genre',onSelect:()=>goScope('genres')},{label:'Playlists',icon:'Playlist',onSelect:()=>goScope('playlists')},{label:'Sparklists',icon:'Sparklist',onSelect:()=>goScope('sparklists')}]}>More</Menu></nav>:<span className="compact-destination-name">{destination==='home'?personalName(preferences.display_name,!loading):'Search'}</span>}
            <Menu label="Create collection" morph="plus" items={[{label:'New playlist',icon:'Playlist',onSelect:()=>createCollection('manual')},{label:'New Sparklist',icon:'Sparklist',onSelect:()=>createCollection('spark')},{label:'New Smart playlist',icon:'SmartPlaylist',onSelect:()=>createCollection('rule')}]}>+</Menu>
          </div>}

          {section === "Library" &&
            !["home", "album", "artist","search"].includes(route.view) && (
              <div className="library-toolbar">
                {!compact&&navigation.history.length > 0 && (
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
                {(findSupported||collectionFind)&&<button className="library-find-action" aria-label="Find in this list" aria-keyshortcuts="Meta+F Control+F" aria-expanded={findSupported?findOpen:undefined} aria-controls={findSupported?"list-find":undefined} onClick={openFind}><Icon name="search"/><span>Find</span><kbd>{/Mac|iPhone|iPad/.test(navigator.platform)?"⌘ F":"Ctrl F"}</kbd></button>}
                {route.view==="artists"&&<ThemedSwitch label="Album Artists" checked={!!preferences.artists_album_only} onChange={artists_album_only=>changePreferences({...preferences,artists_album_only})}/>}
              </div>
            )}
          {section==='Library'&&findSupported&&<ListFind open={findOpen} focusRequest={findRequest} value={route.search} context={findContext} count={findCount} total={findTotal} unit={findUnit} onChange={changeFind} onClose={closeFind}/>}
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
                  onSetup={()=>setOnboardingOpen(true)}
                  preferences={preferences}
                  onChange={changePreferences}
                  error={saveError}
                  cover={playingCover ?? resolvedThemeCover}
                  playback={player}
                  miniTheme={playingStyle}
                  onVolume={volume=>run({type:"volume",volume})}
                  palette={palette}
                  audioContent={<AudioAnalysis library={library}/>}
                  libraryView={settingsLibraryView}
                  onLibraryView={setSettingsLibraryView}
                  onManageCollections={()=>{setSection("Library");goScope("playlists");}}
                  libraryContent={<LibraryTools view={settingsLibraryView==='review'?'review':'sources'} entry={metadataEntry} library={library} copyOnlyNonLibraryDrops={!!preferences.copy_only_non_library_drops} onCopyOnlyNonLibraryDropsChange={copy_only_non_library_drops=>changePreferences({...preferences,copy_only_non_library_drops})} busy={task.running} task={task} playing={player.is_playing} onAdd={copy=>void chooseFolder(copy)} onTask={(kind,albumId)=>startTask(kind,undefined,albumId)} onRefresh={refresh} onError={setError}/>}

                />
              </Suspense></KeepView><KeepView className="search-page-view" active={section==='Library'&&route.view==='search'}><Suspense fallback={<div className="screen-placeholder" aria-label="Loading search"/>}><SuperCommand presentation="page" focusRequest={searchFocus} open recommendations={sparkRecommendations} library={library} collections={collections} isPlaying={player.is_playing} hasTrack={!!player.track} onClose={()=>chooseDestination(previousDestination.current)} onPlay={id=>playIds([id])} onAlbum={a=>{setSection('Library');openAlbum(a);}} onArtist={a=>{setSection('Library');openArtist(a);}} onCollection={openCollection} onNavigate={value=>{if(value==='Settings'||value==='Devices')chooseSection(value);else goScope(value.toLowerCase() as Route['view']);}} onPlayback={type=>void run({type})} onQueue={()=>void showQueue()} onCreate={setCollections}/></Suspense></KeepView><KeepView className="library-view" active={section==='Library'&&route.view!=='search'}><Suspense fallback={<div className="screen-placeholder" aria-label="Loading library view" aria-busy="true"/>}>{content}</Suspense></KeepView>{section === "Devices" ? (<Suspense fallback={<div className="screen-placeholder" aria-label="Loading devices" aria-busy="true"/>}>
                <Devices
                  library={library} collections={collections}
                  busy={task.running}
                  onSync={(root) => startTask("sync", root)}
                />
              </Suspense>) : null}
            </div>
          </main>
          </div>
          {task.message && (task.kind!=='enrich'||section!=='Settings'||settingsLibraryView!=='review'||settingsKey.current!=='Library') && (
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
                        : task.kind==='enrich' ? 'Suggestion check complete' : "Finished"}
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
              {!task.running&&task.kind==='enrich'&&!task.error&&<button onClick={()=>openMetadata()}>Open suggestions</button>}
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
      {fileDrop&&<div className="file-drop-hint" role="status"><Icon name="Song"/><strong>Copy music into {dropTarget.current?.display_name??'musicOS'}</strong><span>Drop songs or folders. Originals stay where they are.</span></div>}
      {playlistTrack&&<PlaylistPicker ids={(playlistTrack as Track&{selectionIds?:number[]}).selectionIds} track={playlistTrack} library={library} items={collections} onChange={setCollections} onClose={()=>setPlaylistTrack(null)}/>}
      {deletingCollection&&<CollectionDelete item={deletingCollection} onChange={setCollections} onClose={()=>setDeletingCollection(null)}/>}
      {creatingCollection==='rule'&&<SmartCreate onChange={setCollections} onClose={()=>setCreatingCollection(null)}/>}
      {creatingCollection&&creatingCollection!=='rule'&&<CollectionCreate kind={creatingCollection} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)} sparkContent={<Suspense fallback={<div className="screen-placeholder" aria-label="Loading Sparklist" aria-busy="true"/>}><SparkCreate initialSeed={sparkSeed} preferences={preferences} onPreferences={changePreferences} playingId={player.track?.id} isPlaying={player.is_playing} tempos={tempos} onToggle={async()=>setPlayer(await call("playback",{action:{type:"toggle"}}))} onPlay={async ids=>{const playable=ids.filter(id=>availableIds.has(id));if(!playable.length)throw Error("No available preview songs.");setPlayer(await call("playback",{action:{type:"play",ids:playable,start_index:0,shuffle:false}}));}} onSetup={()=>{sessionStorage.setItem("musicos-audio-page","spark");window.dispatchEvent(new CustomEvent("musicos-audio-page",{detail:"spark"}));setCreatingCollection(null);captureScroll();sessionStorage.setItem("musicos-settings-tab","Audio");window.dispatchEvent(new CustomEvent("musicos-settings-tab",{detail:"Audio"}));setSection("Settings");}} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)}/></Suspense>}/>}
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
              ? "Keep music in its current location, or make a copy in your musicOS library."
              : "Copy these files into your managed musicOS library. The originals stay where they are."}
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
              Copy into musicOS
            </button>
            <button onClick={() => setPendingImport(null)}>Cancel</button>
          </div>
        </dialog>
      )}
      <QuickAudio theme={playingStyle} player={player} preferences={preferences} onChange={changePreferences} onVolume={volume=>void run({type:"volume",volume})} error={saveError}/>
      <ArtworkViewer depth={!!preferences.artwork_depth}/>
      {audioPanelPresent&&<AudioPanel view={queueView} theme={playingStyle} closing={!queueOpen} player={player} scale={scale} presentation={dockedAudio?'sidebar':'floating'} onClose={()=>setQueueOpen(false)} onAction={run} onSettings={()=>{sessionStorage.setItem('musicos-settings-tab','Audio');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));setQueueOpen(false);setSection('Settings');}}/>}
      {expandedPresent&&<ExpandedPlayer background={preferences.player_background??'flowing'} previewTrack={lyricPreview} onPreviewPlay={async()=>{if(lyricPreview){if(!lyricPreview.available)throw Error('This song is unavailable.');setPlayer(await call('playback',{action:{type:'play',ids:[lyricPreview.id],start_index:0,shuffle:false}}));setLyricPreview(null);}}} onEditLyrics={id=>{setExpandedOpen(false);openMetadata({mode:'lyrics',trackId:id});}} layout={shell.layout} onWindowError={setError} returnFocus={playerOpener.current} open={expandedOpen&&!visualizerOpen} view={playerView} onView={view=>{setPlayerView(view);if(view!=='lyrics')setLyricPreview(null);}} onClose={()=>setExpandedOpen(false)} player={player} cover={playingCover??null} bpm={player.track?tempos.get(player.track.id):undefined} scale={scale} theme={playingStyle} onAction={runImmersive} onFavorite={()=>{if(player.track)void favorite(player.track);}} items={nowActions} origin={playerOrigin.current}/>}
      {dropNotice&&<div className="song-drop-notice" role="status">{dropNotice}</div>}
      {activityOpen&&<Activity onClose={()=>setActivityOpen(false)} onResults={kind=>{
        setActivityOpen(false);
        if(kind==='enrich')openMetadata();
        else if(kind==='wifi')chooseSection('Devices');
        else {sessionStorage.setItem('musicos-settings-tab','Integrations');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Integrations'}));chooseSection('Settings');}
      }}/>}
      {commandPresent&&<Suspense fallback={null}><SuperCommand open={commandOpen} recommendations={sparkRecommendations} library={library} collections={collections} isPlaying={player.is_playing} hasTrack={!!player.track} onClose={()=>setCommandOpen(false)} onPlay={id=>playIds([id])} onAlbum={a=>{setSection('Library');openAlbum(a);}} onArtist={a=>{setSection('Library');openArtist(a);}} onCollection={openCollection} onNavigate={destination=>{if(destination==='Settings'||destination==='Devices')chooseSection(destination);else{setSection('Library');goScope(destination.toLowerCase() as Route['view']);}}} onPlayback={type=>void run({type})} onQueue={()=>void showQueue()} onCreate={setCollections}/></Suspense>}
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
    </div>}</KeepView></ViewActivity>{visualizerOpen&&<VisualizerPlayer preferences={preferences} onPreferences={changePreferences} open view={visualizerView} onView={setVisualizerView} onClose={()=>setVisualizerOpen(false)} returnFocus={visualizerOpener.current} layout={shell.layout} onWindowError={setError} player={player} cover={playingCover??null} bpm={player.track?tempos.get(player.track.id):undefined} scale={scale} theme={playingStyle} onAction={runImmersive} onFavorite={()=>{if(player.track)void favorite(player.track);}} items={nowActions} origin={null} onPreviewPlay={async()=>{}} onEditLyrics={id=>{setVisualizerOpen(false);openMetadata({mode:'lyrics',trackId:id});}}/>}{onboardingOpen&&<Onboarding preferences={preferences} artPalette={artPalette} task={task} songs={library.tracks.length} onImport={setupImport} onFinish={finishSetup} onDismiss={()=>setOnboardingOpen(false)}/>}</></SongPreviewProvider>
  );
}
