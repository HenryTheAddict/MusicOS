import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  IntegrationStatus,
  IntegrationRequest,
  ArtistInfo,
  Bootstrap,
  CatalogItem,
  DeviceStatus,
  Library,
  Palette,
  Playback,
  PlaybackAction,
  Preferences,
  Review,
  SyncPlan,
  Task,
} from "./types";
export const native = isTauri();
const localRequests = new Map<string, Promise<string | null>>();
export function libraryArtwork(trackId: number, perTrack=false) {
  const key=`${trackId}:${perTrack}`;
  let request = localRequests.get(key);
  if (!request) {
    request = call("library_artwork", { trackId, perTrack });
    localRequests.set(key, request);
    request.catch(() => localRequests.delete(key));
  }
  return request;
}
export function invalidateArtwork() {
  localRequests.clear();
  window.dispatchEvent(new Event("musicos-artwork-invalidated"));
}
const cachedArtwork = new Map<string, Promise<string>>();
export function localArtwork(url: string) {
  let request = cachedArtwork.get(url);
  if (!request) {
    request = call("cache_artwork", { url });
    cachedArtwork.set(url, request);
    request.catch(() => cachedArtwork.delete(url));
  }
  return request;
}
export const assets = Object.fromEntries(
  Object.entries(
    import.meta.glob("../../assets/icons/ui/*.svg", {
      eager: true,
      query: "?url",
      import: "default",
    }),
  ).map(([path, url]) => [path.replace("../../", ""), url as string]),
);
export function asset(path: string) {
  return assets[path] ?? "";
}
export function artwork(path: string | null | undefined) {
  if (!path) return asset("assets/icons/ui/noCOVER.svg");
  if (path.startsWith("https://") || path.startsWith("data:")) return path;
  if (path.startsWith("assets/")) return asset(path);
  return native ? convertFileSrc(path) : path;
}
export const emptyLibrary: Library = {
  tracks: [],
  albums: [],
  artists: [],
  genres: [],
  sources: [],
};
export const defaultPreferences: Preferences = {
  version: 1,
  display_name: "Henry",
  interface_percent: 100,
  coverflow_colors: false,
};
export const emptyPlayback: Playback = {
  track: null,
  queue: [],
  queue_index: null,
  is_playing: false,
  position: 0,
  volume: 1,
  shuffle: false,
  repeat: "none",
  error: null,
  backend: "starting",
};
export const emptyTask: Task = {
  running: false,
  kind: "",
  processed: 0,
  total: 0,
  message: "",
  error: null,
  cancelled: false,
};
type Commands = {
  audio_tempos:{args:undefined;result:{running:boolean;tempos:{track_id:number;bpm:number}[]}};
  artwork_performance:{args:{sample:{width:number;height:number;frames:number;interface_percent:number;raf_median_ms:number;raf_p95_ms:number;cpu_draw_p95_ms:number;over_120hz_budget:number}};result:void};
  depth_setup:{args:{action: string};result:import("./types").DepthStatus};
  depth_artwork:{args:{path:string};result:import("./types").DepthArtwork|null};
  portrait_artwork:{args:{path:string};result:string};
  metadata_status: {args:{retry?:boolean};result:import("./types").MetadataStatus};
  album_info:{args:{albumId:string};result:{description:string|null;source_url:string|null}};
  sparklist:{args:{request:import("./types").SparkRequest};result:import("./types").SparkState};
  audio_analysis:{args:{action:'start'|'rescan'|'rescan_all'|'analyze_seed'|'status'|'cancel'|'pause'|'resume';ids?:number[]};result:import('./types').AnalysisStatus};
  playlist_file:{args:{action:'import'|'export'|'artwork';id?:string;path:string};result:{items:import('./types').SavedCollection[];missing:string[];imported:number}};
  organize_files:{args:{action:"preview";token?:string};result:import("./FileOrganizer").OrganizationPreview}|{args:{action:"apply"|"undo"|"recover";token?:string};result:{count:number}};
  relocate_source:{args:{id:number;directory:string};result:{matched:number;unmatched:number}};
  forget_source:{args:{id:number;token?:string};result:{id:number;name:string;path:string;songs:number;favorites:number;memberships:number;metadata?:number;token:string;count?:number}};
  library_source: {args:{id:number;enabled:boolean};result:void};
  audio_specs: {args:{trackId:number};result:{format:string;sample_rate:number|null;bit_depth:number|null;bitrate:number|null;channels:number|null;lossless:boolean}};
  home_activity:{args:undefined;result:import("./types").HomeActivity};
  spark_recommendations:{args:{current:number|null};result:import("./types").SparkRecommendation[]};
  playlist_suggestions:{args:{id:string};result:{ids:number[];state:string;message:string}};
  collections: {args:{request?:import("./types").CollectionRequest};result:import("./types").SavedCollection[]};
  toggle_favorite:{args:{trackId:number};result:boolean};
  metadata_edit: { args: {ids:number[];writeFiles?:boolean;revisions?:[number,number][];patch:Partial<{title:string;artist:string;album:string;album_artist:string;genre:string;year:number;track_number:number;disc_number:number}>}; result:number };
  sync_window_chrome: {args: {centerY:number}; result: void};
  mini_player: {args: {mode:'normal'|'slim'|'tall'|'queue';menuOpen?:boolean;pinned?:boolean}; result:void};
  mini_resize: {args:{width:number;height:number};result:void};
  window_pin: {args:{enabled:boolean};result:void};
  integration_status: { args: undefined; result: IntegrationStatus };
  integration_action: { args: { request: IntegrationRequest }; result: {status: IntegrationStatus; auth_url: string | null} };
  inspect_import: { args: { paths: string[] }; result: boolean };
  library_artwork: { args: { trackId: number; perTrack?:boolean }; result: string | null };
  cache_artwork: { args: { url: string }; result: string };
  bootstrap: { args: undefined; result: Bootstrap };
  library_ready: {args:undefined;result:void};
  bootstrap_state: {args:undefined;result:Omit<Bootstrap,"library">};
  refresh_library: { args: undefined; result: Library };
  save_preferences: { args: { preferences: Preferences }; result: Preferences };
  playback: { args: { action: PlaybackAction }; result: Playback };
  library_task: {
    args: { request: { kind: string; paths?: string[]; root?: string } };
    result: void;
  };
  cancel_task: { args: undefined; result: void };
  album_metadata:{args:{request:{action:string;search?:string;album_id?:string;token?:string;rows?:{track_id:number;fields:string[]}[];write_files?:boolean}};result:{albums?:import('./types').AlbumReview[];count?:number;history_id?:number}};
  metadata_files:{args:{action:string;ids?:number[];id?:string;writeFiles?:boolean};result:{values?:Record<string,unknown>[];files?:{backup_bytes:number;files:{path:string;format?:string;error?:string}[];supported_fields:string[]};jobs?:{id:string;history_id?:number;state:string;bytes:number;files:number}[]}};
  metadata_workspace:{args:{request:import("./types").MetadataWorkspaceRequest};result:import("./types").MetadataWorkspace};
  metadata_review: {
    args: { trackId?: number; accept?: boolean };
    result: Review[];
  };
  artist_info: { args: { name: string;refresh?:boolean }; result: ArtistInfo };
  discover: {
    args: { term: string; kind: string; lookup?: string };
    result: CatalogItem[];
  };
  cancel_search: { args: undefined; result: void };
  artwork_rendition:{args:{path:string;pixels:number};result:string};
  artwork_palette: { args: { path: string }; result: Palette };
  device: {
    args: { action: string; target?: string; root?: string };
    result: DeviceStatus | SyncPlan;
  };
  open_provider: { args: { url: string }; result: void };
};
export async function call<K extends keyof Commands>(
  command: K,
  args?: Commands[K]["args"],
): Promise<Commands[K]["result"]> {
  if (native) return invoke(command, args);
  if (
    import.meta.env.DEV &&
    new URLSearchParams(location.search).has("fixture")
  ) {
    const { fixtureCall } = await import("./fixtures");
    return fixtureCall(command, args) as Promise<Commands[K]["result"]>;
  }
  if (command === "bootstrap" || command === "bootstrap_state")
    return {
      library: emptyLibrary,
      preferences: defaultPreferences,
      playback: emptyPlayback,
      task: emptyTask,
    } as Commands[K]["result"];
  if(command==="refresh_library")return emptyLibrary as Commands[K]["result"];
  throw new Error("Open the MusicOS desktop app to use your library.");
}
export function errorText(error: unknown) {
  if (error && typeof error === "object" && "message" in error)
    return String(error.message);
  const text=String(error);return /database (?:is |file is )?locked|database is busy|SQLITE_BUSY/i.test(text)?"Your library is finishing another change. Wait a moment, then retry.":text;
}
export async function subscribe<T>(
  event: string,
  callback: (payload: T) => void,
) {
  return native ? listen<T>(event, (e) => callback(e.payload)) : () => {};
}
