import LibraryFilter,{defaultFilters,filterAlbums} from './LibraryFilter';
import {registerAlbumArtwork} from './albumArtwork';
import QuickAudio from "./QuickAudio";
import AudioPanel from "./AudioPanel";
import ArtworkViewer from "./ArtworkViewer";
import AlbumArtwork from './AlbumCoverMotion';
import AlbumArtists from "./AlbumArtists";
import CollectionCreate from "./CollectionDesign";
import SparkCreate from "./SparkCreate";
import VolumeControl from "./VolumeControl";
import {
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
  invalidateArtwork,
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
import CollectionManager from './CollectionManager';
import AudioAnalysis from './AudioAnalysis';
import Settings from "./Settings";
import ArtistOverview from "./ArtistOverview";
import { resolveArtworkTheme, clearThemeCache } from "./artworkTheme";
import { useNativeChrome } from "./nativeChrome";
import Devices from "./Devices";
import LibraryTools from "./LibraryTools";
import { useJellyMotion, usePageEntrance, useOverscrollFeedback } from "./motion";
import { useSmoothCorners } from "./smoothCorners";
import PlaybackProgress from "./PlaybackProgress";
import AudioQuality from "./AudioQuality";
import AlbumDescription from "./AlbumDescription";
import SuperCommand from "./SuperCommand";
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
  function openManagement(view:"sources"|"review"|"playlists"|"sparklists",ids:number[]=[]){captureScroll();setSettingsLibraryView(view);setMetadataIds(ids);sessionStorage.setItem("musicos-settings-tab","Library");setSection("Settings");}
  const [collections,setCollections]=useState<SavedCollection[]>([]);
  const [creatingCollection,setCreatingCollection]=useState<"manual"|"spark"|null>(null);
  const [collectionError,setCollectionError]=useState("");
  const favoritePending=useRef(new Map<number,Promise<void>>());

  const [library, setRawLibrary] = useState<Library>(emptyLibrary);
  const setLibrary:typeof setRawLibrary=next=>setRawLibrary(old=>{const value=typeof next==='function'?next(old):next;registerAlbumArtwork(value);return value;});
  const tracksById=useMemo(()=>new Map(library.tracks.map(t=>[t.id,t])),[library.tracks]);
  const [preferences, setPreferences] = useState(defaultPreferences);
  useSmoothCorners();
  useNativeChrome();
  const [rawPlayer, setPlayer] = useState<Playback>(emptyPlayback);
  const player=useMemo(()=>({...rawPlayer,track:rawPlayer.track?(tracksById.get(rawPlayer.track.id)??rawPlayer.track):null,queue:rawPlayer.queue.map(t=>tracksById.get(t.id)??t)}),[rawPlayer,tracksById]);
  const [task, setTask] = useState<Task>(emptyTask);
  const [loading, setLoading] = useState(true);
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
  const [pendingImport, setPendingImport] = useState<{
    paths: string[];
    folders: boolean;
  } | null>(null);
  const importDialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (pendingImport) importDialog.current?.showModal();
  }, [pendingImport]);
  const [queueOpen, setQueueOpen] = useState(false);
  useEffect(()=>{const open=()=>{setQueueOpen(false);setSection('Settings');sessionStorage.setItem('musicos-settings-tab','Audio');sessionStorage.setItem('musicos-audio-page','quality');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));window.dispatchEvent(new CustomEvent('musicos-audio-page',{detail:'quality'}));};window.addEventListener('musicos-open-audio-settings',open);return()=>window.removeEventListener('musicos-open-audio-settings',open);},[]);
  const [commandOpen,setCommandOpen]=useState(false);
  const [artistLoading,setArtistLoading]=useState<Set<string>>(()=>new Set());
  const artistCompletions=useRef(new Map<string,number>());
  const [artistInfo, setArtistInfo] = useState<ArtistInfo | null>(null);
  const [resolvedThemeCover, setResolvedThemeCover] = useState<string | null>(null);
  const [themeEpoch, setThemeEpoch] = useState(0);
  useEffect(() => { const clear = () => { clearThemeCache(); setThemeEpoch(n=>n+1); }; window.addEventListener("musicos-artwork-invalidated", clear); return () => window.removeEventListener("musicos-artwork-invalidated", clear); }, []);
  const [palette, setPalette] = useState<Palette | null>(null);
  const [previousPalette, setPreviousPalette] = useState<Palette | null>(null);
  useLayoutEffect(()=>{
    const tokens={'--accent':palette?.accent??'#0064df','--surface-color':palette?.background??'#cbf1ff','--blob-color':palette?.blob??'#54b9ff','--ink':palette?.text??'#111820','--accent-ink':contrastInk(palette?.accent??'#0064df')};
    for(const [name,value] of Object.entries(tokens))document.documentElement.style.setProperty(name,value);
  },[palette]);
  const [navigation, dispatch] = useReducer(navigationReducer, undefined, () =>
    restoreNavigation(sessionStorage.getItem("musicos-navigation-v1")),
  );
  const route = navigation.current;
  const scrollRef = useRef<HTMLElement>(null);
  useJellyMotion();
  useOverscrollFeedback(scrollRef,!loading);
  usePageEntrance(scrollRef, `${section}:${route.view}:${route.id ?? ""}`, !loading);
  const savePending = useRef<Preferences|null>(null);
  const savingPreferences=useRef(false);
  const scale = preferences.interface_percent / 100;
  const mounted = useRef(true);
  const refresh = useCallback(async () => {
    try {
      const next=await call("refresh_library");
      setLibrary(next);
      void call("collections",{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));
      dispatch({type:"reconcile",albums:next.albums.map(a=>a.id),artists:next.artists.map(a=>a.id),genres:next.genres.map(g=>g.id)});
      invalidateArtwork();
    } catch (e) {
      setError(errorText(e));
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    let cleanups: (() => void)[] = [];
    let gone = false;
    void call("bootstrap")
      .then((data) => {
        if (gone) return;
        setLibrary(data.library);
        void call("collections",{}).then(setCollections).catch(e=>setCollectionError(errorText(e)));
        setPreferences(data.preferences);
        setPlayer(data.playback);
        setTask(data.task);
        if (data.preference_error) setSaveError(data.preference_error);
      })
      .catch((e) => setError(errorText(e)))
      .finally(() => {
        if (!gone) setLoading(false);
      });
    async function events() {
      const listeners = await Promise.all([
        subscribe<Partial<Playback>>("playback", (update) =>
          setPlayer((old) => ({ ...old, ...update })),
        ),
        subscribe<Task>("task", setTask),
        subscribe("library-changed", () => void refresh()),
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
    for (const a of library.albums) {
      for (const id of library.artists.filter(artist=>artist.album_ids.includes(a.id)).map(artist=>artist.id)) {
      if (
        !covers.has(id) ||
        (!covers.get(id)?.cover && a.cover)
      )
        covers.set(id, a);
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
  useEffect(() => {
    let gone=false;
    async function resolve() {
      const artistFallback=section==="Library" && route.view==="artist" ? firstArtistAlbum?.cover : null;
      const result=preferences.coverflow_colors ? await resolveArtworkTheme([source,artistFallback],themeTrackId) : null;
      if(!gone){setResolvedThemeCover(result?.path??null);setPalette(old=>{setPreviousPalette(old);return result?.palette??null;});}
    }
    void resolve();return ()=>{gone=true;};
  }, [preferences.coverflow_colors, source, themeTrackId, firstArtistAlbum?.cover, route.view, section, themeEpoch]);
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
    if (section === "Library" && scrollRef.current)
      dispatch({
        type: "patch",
        patch: { scroll: scrollRef.current.scrollTop },
      });
  }
  function goBack() {
    captureScroll();
    dispatch({ type: "back" });
  }
  function chooseSection(next: "Library" | "Devices" | "Settings") {
    if(next==='Settings') { sessionStorage.setItem('musicos-settings-tab','Appearance');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Appearance'})); }
    captureScroll();
    setSection(next);
  }
  function openRoute(next: Route) {
    captureScroll();
    dispatch({ type: "open", route: next });
  }
  function goScope(view: Route["view"]) {
    captureScroll();
    dispatch({ type: "scope", view });
  }
  function openAlbum(a: Album) {
    openRoute({ view: "album", id: a.id, search: "", scroll: 0 });
  }
  function openArtist(a: Artist) {
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
        .map((id) => library.tracks.find((t) => t.id === id)!)
        .filter(Boolean);
    if (route.view === "artist")
      records = records.filter((t) => artist?.track_ids.includes(t.id));
    if(route.view==="playlist")records=(collections.find(c=>c.id===route.id)?.track_ids??[]).map(id=>library.tracks.find(t=>t.id===id)!).filter(Boolean);
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
      onClick: () => openAlbum(a),
      menu: [
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
  const rows = (records: Track[]) => (
    <TrackRows
      tracks={records}
      showDurations={preferences.show_song_durations!==false}
      albumArtist={route.view==='album'?album?.artist:undefined}
      levels={player.audio_levels}
      scrollRef={scrollRef}
      scale={scale}
      initialOffset={route.scroll}
      playingId={player.track?.id}
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
      onEdit={track=>openManagement("review",[track.id])}
      onFavorite={track=>void favorite(track)}
      playlistActions={track=>[{label:"Play next",icon:"Song",disabled:!track.available,onSelect:()=>void run({type:"play_next",id:track.id})},{label:"Add to Up next",icon:"Song",disabled:!track.available,onSelect:()=>void run({type:"add_to_queue",id:track.id})},...collections.filter(c=>!c.spark).map(c=>({label:`${c.track_ids.includes(track.id)?'Remove from':'Add to'} ${c.name}`,icon:'Song',onSelect:()=>void call('collections',{request:{action:c.track_ids.includes(track.id)?'remove':'add',id:c.id,track_ids:[track.id]}}).then(setCollections).catch(e=>setError(errorText(e)))}))]}
    />
  );
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
  const paletteStyle = (p: Palette | null): CSSProperties =>
    p
      ? {
          backgroundColor: p.background,
          backgroundImage: `radial-gradient(ellipse at 15% 25%, ${p.blob}55, transparent 65%),radial-gradient(ellipse at 90% 90%, ${p.blob}33, transparent 65%)`,
        }
      : { backgroundColor: section === "Library" ? "#00aeff" : "#c9efff" };
  let content;
  if (loading)
    content = (
      <div className="loading" role="status">
        Opening your library…
      </div>
    );
  else if (route.view === "album" && album)
    content = (
      <article className="album-detail">
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
      <article className="artist-detail">
        <ArtistOverview scrollRef={scrollRef} depth={!!preferences.artwork_depth&&preferences.artwork_parallax!==false} key={artist.id} artist={artist} albums={albums} tracks={tracks} info={artistInfo}
          fallback={fallbackAlbum} actions={collectionActions(artist.track_ids)}
          onProvider={url=>void call("open_provider",{url}).catch(e=>setError(errorText(e)))} />
        <h2 className="section-title">Albums</h2>
        <div className="album-strip">
          {albumItems(albums).map((item) => (
            <button
              key={item.id}
              aria-label={item.title}
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
    content=item?<Detail key={item.id} item={item} items={collections} library={library} onChange={setCollections} onDeleted={()=>navigation.history.length?goBack():goScope('albums')} scale={scale} actions={collectionActions(tracks.map(t=>t.id))}>{rows(tracks)}</Detail>:<Empty title="Playlist unavailable">It may have been removed.</Empty>;
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
      library.artists
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
          onClick: () => openArtist(a),
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
  const displayScope = (
    ["album", "artist", "genre"].includes(route.view)
      ? `${route.view}s`
      : route.view
  ) as Scope;
  return (
    <div
      className="app-shell"
      data-size={preferences.interface_percent}
      data-adaptive={!!palette}
      style={
        {
          "--accent": palette?.accent ?? "#0064df",
          "--shell-color": palette?.blob ?? "#00baff",
          "--surface-color": palette?.background ?? "#cbf1ff",
          "--blob-color": palette?.blob ?? "#54b9ff",
          "--ink": palette?.text ?? "#111820",
          "--accent-ink": palette?.accent && contrastInk(palette.accent),
        } as CSSProperties
      }
    >
      <a className="skip-link" href="#library-content">
        Skip to content
      </a>
      <header className="app-header">
        <div
          className="window-drag-region"
          data-tauri-drag-region
        ><div className="header-brand" data-tauri-drag-region><Icon name="Album"/><span>MusicOS</span></div></div>
        <Menu
          label="Choose section"
          items={(["Library", "Devices", "Settings"] as const).map((label) => ({
            label,
            onSelect: () => chooseSection(label),
          }))}
        >
          <span className="section-name">{section}</span>
          <span className="chevron" aria-hidden="true"><svg viewBox="0 0 24 12"><path d="M1 1 12 10 23 1" /></svg></span>
        </Menu>
        <div className="header-drag-spacer" data-tauri-drag-region />
        <section className="now-playing" aria-label="Now playing">
          <button
            className="now-playing-cover"
            aria-label="Show playing album"
            onClick={() => {
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
            <strong key={player.track?.id ?? "empty"} title={player.track?.title}>
              {player.track?.title ?? "Nothing playing yet"}
            </strong>
            <span>
              {player.track
                ? `${player.track.artist} · ${player.track.album}`
                : "Your next favorite is waiting."}
            </span>
            <PlaybackProgress showTime={preferences.show_now_playing_time!==false} transition={player.dj_transition} trackId={player.track?.id} position={player.position}
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
              {navigation.history.length>0?<button className="sidebar-back" onClick={goBack}><Icon name="Back"/><span>Back to {(() => {const view=navigation.history.at(-1)!.view;return ['album','artist','genre'].includes(view)?view:view[0].toUpperCase()+view.slice(1);})()}</span></button>:<label className="search sidebar-search"><Icon name="search"/><input aria-label="Search" placeholder="Search" value={route.search} onChange={e=>{dispatch({type:'patch',patch:{search:e.target.value,scroll:0}});if(scrollRef.current)scrollRef.current.scrollTop=0;}}/></label>}
              <button className="command-shortcut" aria-label="Open SuperCommand" title="SuperCommand (⌘K / Ctrl+K)" onClick={()=>setCommandOpen(true)}><span aria-hidden="true" className="command-symbol">⌘</span></button>
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
              <div className="collections-heading"><h2 title={`${preferences.display_name}'s stuff`}>
                {preferences.display_name}'s stuff
              </h2><button aria-label="Manage collections" onClick={()=>{setSection('Library');goScope('playlists');}}><Icon name="Playlist"/></button></div>
              <button
                className={route.view === "favorites" ? "active" : ""}
                onClick={() => goScope("favorites")}
              >
                <Icon name="hearted" />
                <span>
                  <strong>Favorites</strong>
                  <small>
                    {library.tracks.filter((t) => t.favorited).length} songs
                  </small>
                </span>
              </button>
              <div className="personal-collections">{collections.map(item=>{
                const members=item.track_ids.map(id=>tracksById.get(id)).filter(Boolean);
                const icon=item.kind==='audio'?'Sparklist':'Playlist';
                return <div key={item.id} className={`personal-collection ${route.id===item.id?'active':''}`}>
                  <button className="collection-open" onClick={()=>openCollection(item)}><PlaylistArtwork item={item} library={library}/><span className="collection-label"><strong><span>{item.name}</span><Icon name={icon}/></strong><small>{item.kind==='rule'?'Smart playlist · ':''}{item.track_ids.length} {item.track_ids.length===1?'song':'songs'} · {collectionDuration(members.reduce((sum,t)=>sum+(t?.duration??0),0))}</small></span></button>
                  <button className="collection-shuffle" aria-label={`Shuffle ${item.name}`} disabled={!members.length} onClick={()=>playIds(item.track_ids,true)}><Icon name="CollectionShuffle"/></button>
                </div>;
              })}</div>
              {collectionError&&<small role="alert">{collectionError}</small>}
            </section>
            <footer className="sidebar-footer">
              <span>{library.tracks.length} songs</span>
              <span>
                {collectionDuration(
                  library.tracks.reduce((s, t) => s + t.duration, 0),
                )}
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
              </div>
            )}
          <div className="page-frame">
            <div
              className="theme-layer previous-theme"
              style={paletteStyle(previousPalette)}
            />
            <div
              className="theme-layer current-theme"
              key={`${source ?? "blue"}:${palette?.background ?? "default"}:${section}`}
              style={paletteStyle(palette)}
            />
            <main
              id="library-content"
              className={`page-scroll ${section.toLowerCase()}-page`}
              ref={scrollRef}
            >
            <div className="page-content">
              {section === "Settings" ? (
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
                  libraryContent={<LibraryTools key={`${settingsLibraryView}:${metadataIds.join(',')}`} view={settingsLibraryView==='review'?'review':'sources'} initialIds={metadataIds} library={library} busy={task.running} onAdd={copy=>void chooseFolder(copy)} onTask={startTask} onRefresh={refresh} onError={setError}/>}

                />
              ) : section === "Devices" ? (
                <Devices
                  busy={task.running}
                  onSync={(root) => startTask("sync", root)}
                />
              ) : (
                content
              )}
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
      {creatingCollection&&<CollectionCreate kind={creatingCollection} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)} sparkContent={<SparkCreate preferences={preferences} onPreferences={changePreferences} playingId={player.track?.id} isPlaying={player.is_playing} onToggle={async()=>setPlayer(await call("playback",{action:{type:"toggle"}}))} onPlay={async ids=>{const playable=ids.filter(id=>availableIds.has(id));if(!playable.length)throw Error("No available preview songs.");setPlayer(await call("playback",{action:{type:"play",ids:playable,start_index:0,shuffle:false}}));}} onSetup={()=>{sessionStorage.setItem("musicos-audio-page","spark");window.dispatchEvent(new CustomEvent("musicos-audio-page",{detail:"spark"}));setCreatingCollection(null);captureScroll();sessionStorage.setItem("musicos-settings-tab","Audio");window.dispatchEvent(new CustomEvent("musicos-settings-tab",{detail:"Audio"}));setSection("Settings");}} library={library} items={collections} onChange={setCollections} onOpen={openCollection} onClose={()=>setCreatingCollection(null)}/>}/>}
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
      <QuickAudio player={player} preferences={preferences} onChange={changePreferences} onVolume={volume=>void run({type:"volume",volume})} error={saveError}/>
      <ArtworkViewer depth={!!preferences.artwork_depth}/>
      {queueOpen&&<AudioPanel player={player} scale={scale} presentation={preferences.audio_presentation??'sidebar'} onClose={()=>setQueueOpen(false)} onAction={action=>void run(action)} onSettings={()=>{sessionStorage.setItem('musicos-settings-tab','Audio');window.dispatchEvent(new CustomEvent('musicos-settings-tab',{detail:'Audio'}));setQueueOpen(false);setSection('Settings');}}/>}
      {commandOpen&&<SuperCommand library={library} collections={collections} isPlaying={player.is_playing} hasTrack={!!player.track} onClose={()=>setCommandOpen(false)} onPlay={id=>playIds([id])} onAlbum={a=>{setSection('Library');openAlbum(a);}} onArtist={a=>{setSection('Library');openArtist(a);}} onCollection={openCollection} onNavigate={destination=>{if(destination==='Settings'||destination==='Devices')chooseSection(destination);else{setSection('Library');goScope(destination.toLowerCase() as Route['view']);}}} onPlayback={type=>void run({type})} onQueue={()=>setQueueOpen(true)} onCreate={setCollections}/>}
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
    </div>
  );
}
