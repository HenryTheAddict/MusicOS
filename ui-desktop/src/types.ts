export interface AudioQualityInfo {format:string;lossless:boolean;sample_rate:number|null;bit_depth:number|null;bitrate:number|null;channels:number|null}
export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  track_number: number;
  disc_number: number;
  duration: number;
  genre: string;
  year: number;
  stable_id: string;
  favorited: boolean;
  available: boolean;
  cover_art_path: string | null;
  album_artist?:string|null;
  release_id?:string|null;
  audio_quality?:AudioQualityInfo|null;
}
export interface Album {
  legacy_ids?:string[];
  release_type?:string|null;
  release_type_source?:string|null;
  release_track_total?:number|null;
  id: string;
  artist_id: string;
  title: string;
  artist: string;
  genre: string;
  year: number;
  cover: string | null;
  track_ids: number[];
  duration: number;
}
export interface Artist {
  legacy_ids?:string[];
  credited_names?:string[];
  id: string;
  name: string;
  portrait: string | null;
  album_ids: string[];
  track_ids: number[];
}
export interface Genre {
  id: string;
  name: string;
  album_ids: string[];
  track_ids: number[];
}
export interface Source {song_count?:number;
  id: number;
  kind?: string;
  enabled?: boolean;
  display_name: string;
  path: string;
  available: boolean;
  last_error: string | null;
}
export interface Library {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  genres: Genre[];
  sources: Source[];
  protected_source_ids?:number[];
}
export interface LibraryFilters {sort:'recent'|'title';genre:string;quality:'all'|'lossless'|'lossy'|'mixed';favorites:boolean;availability:'all'|'available'|'unavailable';separate_singles:boolean}
export interface Listening {bands:3|8;gains3:number[];gains8:number[];speed:number;pitch:number;linked:boolean;reverb?:number;reverb_depth?:number}
export interface Appearance {mode:"system"|"light"|"dark";source:"artwork"|"blue"|"custom";accent:string;saturation:number;tint:number}
export interface Preferences {
  mini_player_view?:"artwork"|"queue";
  now_playing_artwork_action?:"album"|"mini_player"|"artwork";
  appearance?:Appearance;
  artists_album_only?:boolean;
  listening?:Listening;
  library_filters?:LibraryFilters;
  version: number;
  display_name: string;
  interface_percent: 80 | 100 | 125 | 150;
  coverflow_colors: boolean;
  artwork_parallax?:boolean;
  artwork_depth?:boolean;
  show_song_durations?:boolean;
  show_now_playing_time?:boolean;
  sparklist_beta?:boolean;
  audio_presentation?: "sidebar" | "floating";
  dj_config?:{enabled:boolean;mode:"smart_transition"|"full_song";character:"subtle"|"creative";preserve_energy:boolean};
}
export interface AudioFeatures {track_id:number;bpm:number|null;bpm_confidence:number;key:string|null;key_index:number|null;key_confidence:number;structure?:StructuralAnalysis|null}
export interface AnalysisStatus {running:boolean;paused:boolean;processed:number;total:number;features:AudioFeatures[];errors:string[]}
export interface Playback {
  listening?:Listening;
  listening_applied?:boolean;applied_reverb?:number;applied_reverb_depth?:number;reverb_processed_frames?:number;applied_listening_revision?:number;requested_listening_revision?:number;transition_outcome?:{request_id:number;outgoing_entry:number;incoming_entry:number;status:string;reason:string}|null;failed_entries?:number[];
  audio_levels?:number[];
  dj_transition?:{plan_id?:number;outgoing_entry:number;incoming_entry:number;status?:"preparing"|"scheduled"|"active";active:boolean;outgoing_cue:number;incoming_cue:number;seconds:number;source_seconds?:number;tempo:number}|null;
  queue_entry_ids?:number[];
  pinned_entries?:number[];
  arranged_entries?:number[];
  dj_mode?:boolean;
  track: Track | null;
  queue: Track[];
  queue_index: number | null;
  is_playing: boolean;
  position: number;
  volume: number;
  shuffle: boolean;
  repeat: string;
  error: string | null;
  backend: string;
}
export interface Task {
  running: boolean;
  kind: string;
  processed: number;
  total: number;
  message: string;
  error: string | null;
  cancelled: boolean;
}
export interface Bootstrap {
  preference_error?: string | null;
  library: Library;
  preferences: Preferences;
  playback: Playback;
  task: Task;
}
export interface ArtistInfo {
  lookup_status?:string;
  biography: string | null;
  portrait: string | null;
  source_url: string | null;
  artist_mbid: string | null;
  portrait_source_url?: string | null;
  portrait_credit?: string | null;
  portrait_license?: string | null;
}
export interface CatalogItem {
  id: string;
  kind: "album" | "artist" | "song";
  title: string;
  artist: string;
  artist_id: string | null;
  album_id: string | null;
  album: string | null;
  cover: string | null;
  genre: string | null;
  duration: number | null;
  track_number: number | null;
  store_url: string | null;
  local: false;
}
export interface Palette {
  background: string;
  accent: string;
  blob: string;
  text: string;
}
export interface Review {candidate_identity?:string;
  track_id: number;
  current_title: string;
  current_artist: string;
  current_album: string;
  revision?:number; candidate_token?:string;current_year?:number|null;current_artwork?:string|null;
  candidate: {
    year?:number|null;artwork_path?:string|null;recording_mbid?:string;release_mbid?:string|null;
    title: string;
    artist: string;
    album: string;
    confidence: number;
  };
}
export interface DeviceStatus {
  target: string;
  connected: boolean;
  error: string | null;
  status: unknown;
  preview: { running: boolean; last_error: string | null };
}
export interface SyncPlan {
  root: string;
  files: number;
  bytes: number;
  target: string;
  profile: string;
}
export type PlaybackAction =
  | { type: "play"; ids: number[]; start_index: number; shuffle: boolean }
  | { type: "toggle" | "previous" | "next" | "shuffle" | "repeat" }
  | { type: "play_next"; id: number }
  | { type: "add_to_queue"; id: number; index?:number }
  | { type: "seek"; position: number }
  | { type: "volume"; volume: number }
  | { type: "select" | "remove"; index: number }
  | { type:'move';from:number;to:number }
  | { type:'clear_upcoming' | 'dj_order' }
  | {type:'dj_mode';enabled:boolean};
export type Scope = "albums" | "artists" | "genres" | "songs" | "playlists" | "sparklists";
export type View =
  | Scope
  | "home"
  | "album"
  | "artist"
  | "genre"
  | "playlist"
  | "favorites"
  | "sources"
  | "review"
  | "discover"
  | "catalog";
export interface Route {
  view: View;
  id?: string;
  search: string;
  scroll: number;
  selected?: string;
  catalog?: CatalogItem;
}

export interface IntegrationConfig {
  discord_enabled: boolean;
  discord_client_id: string;
  lastfm_enabled: boolean;
  lastfm_username: string | null;
}
export interface IntegrationStatus {
  config: IntegrationConfig;
  discord: string;
  lastfm: string;
  queued: number;
}
export type IntegrationRequest =
  | { action: "configure"; discord_enabled: boolean; discord_client_id: string; lastfm_enabled: boolean }
  | { action: "authorize"; api_key: string; shared_secret: string }
  | { action: "finish_authorization" }
  | { action: "disconnect_lastfm" };

export interface SparkDefinition {name:string;match_mode:'all'|'any';rules:{field:string;op:string;value:string}[];limit:number|null}
export interface HomeActivity {frequent:{id:number;plays:number}[];recent:number[]}
export interface SparkRecommendation {id:string;reason:string}
export interface SavedCollection {updated_at?:number;id:string;name:string;track_ids:number[];spark:SparkDefinition|null;kind?:'manual'|'rule'|'audio';description?:string;artwork?:string|null;artwork_choice?:string|null;target_count?:number|null}
export interface CollectionRequest {action:'length'|'cover'|'create'|'rename'|'describe'|'delete'|'add'|'remove'|'reorder'|'create_spark'|'update_spark';id?:string;name?:string;track_ids?:number[];definition?:SparkDefinition;description?:string;artwork_choice?:string;target_count?:number}

export interface SparkGeneration {seed:number|null;mood:string|null;less_like:number[];variant:number;target_count:number}
export interface SparkPreview {track_ids:number[];analyzed:number;available:number;generation_version:string;configuration:SparkGeneration}
export interface SparkState {
 installed?:boolean;generated_artwork?:string|null;analysis?:{running:boolean;paused:boolean;processed:number;total:number};staged_available?:boolean;beta?:boolean;installation?:{running:boolean;copied:number;total:number;error:string|null};ready:boolean;reason:string|null;download_bytes:number;analyzed:number;available:number;analyzed_ids:number[];preview:SparkPreview|null;items:SavedCollection[];chosen_id:number|null}
export type SparkRequest={action:'status'|'choose'|'cancel_install'|'remove_model'|'install_staged'}|{action:'install';directory:string}|{action:'preview';generation:SparkGeneration}|{action:'save';generation:SparkGeneration;name:string;artwork_choice:string;preview?:NonNullable<SparkState["preview"]>}|{action:'refresh';id:string}|{action:'as_playlist';id:string;name:string};
export interface StructuralAnalysis {version:string;duration:number;energy:number;energy_step:number;tempo?:number|null;envelope:number[];beats:number[];beat_confidence:number;phrases:{seconds:number;confidence:number}[];changes:{seconds:number;confidence:number}[];regions:{start:number;end:number;repeated:boolean;confidence:number}[];silence:{start:number;end:number;repeated:boolean;confidence:number}[]}

export type MetadataStatus = {cached:number;checking:number;unavailable:number;retryable:number};

export type DepthStatus={supported:boolean;ready:boolean;running:boolean;downloaded:number;total:number;error:string|null;revision:string;attribution:string};

export type DepthArtwork={photo:string;map:string;full_map?:string;identity?:string;fresh?:boolean};

export interface MetadataHistory {id:number;created_at:number;undone:boolean;conflict:boolean;changes:{track_id:number;before:Record<string,unknown>;after:Record<string,unknown>;current:Record<string,unknown>|null;conflict:boolean}[]}
export interface MetadataWorkspace {lookup?:Record<string,number>;items?:Review[];total?:number;offset?:number;history?:MetadataHistory[]}
export interface MetadataWorkspaceRequest {action:'list'|'apply'|'dismiss'|'ignore'|'ignored'|'restore'|'history'|'undo';search?:string;offset?:number;id?:number;trackId?:number;revision?:number;token?:string;fingerprint?:string;fields?:string[]}

export interface AlbumReview{album_id:string;title:string;artist:string;cover?:string;resolution?:number[];token:string;rows:{track_id:number;current:Record<string,unknown>;candidate:Record<string,unknown>|null;candidate_token?:string;art_resolution?:number[]}[]}
