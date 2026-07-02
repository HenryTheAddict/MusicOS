import { useEffect, useMemo, useState } from "react";
import {
  Album,
  BadgeCheck,
  Cable,
  ChevronDown,
  CircleStop,
  Disc3,
  Download,
  FolderOpen,
  HardDrive,
  Heart,
  ListMusic,
  Loader2,
  MonitorPlay,
  Music2,
  Pause,
  Play,
  Plus,
  RefreshCcw,
  Repeat2,
  Search,
  Settings,
  Shuffle,
  SkipBack,
  SkipForward,
  SlidersHorizontal,
  Trash2,
  Unplug,
  Usb,
  Wand2
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Toaster } from "@/components/ui/sonner";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

type Section =
  | "library"
  | "albums"
  | "artists"
  | "songs"
  | "favorites"
  | "playlists"
  | "devices"
  | "sync"
  | "updates"
  | "settings";

type Track = {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  track_number: number;
  duration: number;
  indexed_at: number;
  cover_art_updated_at?: number | null;
  favorited: boolean;
  play_count?: number;
};

type Playlist = { id: number; name: string };
type Mix = { id: string; name: string; kind: string; track_count: number };
type Config = { target: string; syncDir: string; dbPath: string; emulator?: EmulatorStatus };
type EmulatorStatus = {
  running: boolean;
  starting?: boolean;
  pid?: number | null;
  target?: string;
  dbPath?: string;
  syncDir?: string;
  binPath?: string;
  lastExit?: { code: number | null; signal: string | null; at: string } | null;
};
type SyncState = {
  running: boolean;
  found: number;
  uploaded: number;
  skipped: number;
  failed: number;
  current?: string;
  cancelRequested?: boolean;
  errors?: string[];
  failedFiles?: string[];
  startedAt?: string;
  finishedAt?: string;
};
type LibraryState = {
  status?: string;
  counts?: Record<string, number>;
  playlists?: Playlist[];
  player?: { is_playing?: boolean; position?: number; track?: Track | null };
  queue?: Track[];
  queue_index?: number | null;
  scan_status?: string;
  scan?: { state?: string; indexed?: number; discovered?: number; elapsed_ms?: number; last_error?: string | null };
  usb_status?: string;
  db_path?: string;
  settings_path?: string;
  frame_ms?: number;
  frame_timing?: { p95_ms?: number; slow_frames?: number };
  cover_art_cache_count?: number;
  indexed_roots?: string[];
  audio?: unknown;
};
type SettingsState = {
  user_name?: string;
  device_name?: string;
  theme?: string;
  eq_preset?: string;
  audio_backend?: string;
  alsa_device?: string;
  volume?: number;
  brightness?: number;
  shuffle?: boolean;
  loop_mode?: string;
  library_sort?: string;
  library_group?: string;
  motion_mode?: string;
  seek_step_seconds?: number;
  resume_queue_enabled?: boolean;
};
type LogLine = { at?: string; source?: string; line?: string };

declare global {
  interface Window {
    musicos?: {
      getConfig: () => Promise<Config>;
      setTarget: (target: string) => Promise<Config>;
      chooseFolder: () => Promise<Config>;
      openFolder: () => Promise<unknown>;
      openCoverArtCache: () => Promise<unknown>;
      resetDb: () => Promise<Config>;
      exportDiagnostics: () => Promise<{ canceled?: boolean; path?: string }>;
      backupLibrary: () => Promise<{ canceled?: boolean; path?: string }>;
      restoreLibrary: () => Promise<{ canceled?: boolean; restoredSettings?: boolean; restoredPlaylists?: number }>;
      exportPlaylists: () => Promise<{ canceled?: boolean; count?: number }>;
      importPlaylists: () => Promise<{ canceled?: boolean; imported?: number }>;
      sync: {
        start: () => Promise<SyncState>;
        status: () => Promise<SyncState>;
        dryRun: () => Promise<Record<string, unknown>>;
        retryFailed: () => Promise<SyncState & { retryStarted?: boolean }>;
        cancel: () => Promise<SyncState>;
        history: () => Promise<SyncState[]>;
        clearHistory: () => Promise<SyncState[]>;
      };
      emulator: {
        start: () => Promise<EmulatorStatus>;
        stop: () => Promise<EmulatorStatus>;
        restart: () => Promise<EmulatorStatus>;
        status: () => Promise<EmulatorStatus>;
        logs: () => Promise<{ logs?: LogLine[]; status?: EmulatorStatus }>;
        clearLogs: () => Promise<{ logs?: LogLine[]; status?: EmulatorStatus }>;
      };
    };
  }
}

const defaultConfig: Config = {
  target: "http://127.0.0.1:18080",
  syncDir: "./music",
  dbPath: "out/desktop-media.sqlite3"
};

const sections: Array<{ id: Section; label: string; icon: React.ElementType; group: string }> = [
  { id: "library", label: "Library", icon: Music2, group: "Library" },
  { id: "albums", label: "Albums", icon: Album, group: "Library" },
  { id: "artists", label: "Artists", icon: Disc3, group: "Library" },
  { id: "songs", label: "Songs", icon: ListMusic, group: "Library" },
  { id: "favorites", label: "Favorites", icon: Heart, group: "Library" },
  { id: "playlists", label: "Playlists", icon: Plus, group: "Playlists" },
  { id: "devices", label: "Devices", icon: HardDrive, group: "Device" },
  { id: "sync", label: "Sync", icon: RefreshCcw, group: "Device" },
  { id: "updates", label: "Updates", icon: Download, group: "Device" },
  { id: "settings", label: "Settings", icon: Settings, group: "System" }
];

export function App() {
  const [section, setSection] = useState<Section>("library");
  const [config, setConfig] = useState<Config>(defaultConfig);
  const [library, setLibrary] = useState<LibraryState | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [playlistTracks, setPlaylistTracks] = useState<Track[]>([]);
  const [selectedPlaylistId, setSelectedPlaylistId] = useState<number | null>(null);
  const [mixes, setMixes] = useState<Mix[]>([]);
  const [settings, setSettings] = useState<SettingsState>({});
  const [sync, setSync] = useState<SyncState | null>(null);
  const [syncHistory, setSyncHistory] = useState<SyncState[]>([]);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [query, setQuery] = useState("");
  const [targetDraft, setTargetDraft] = useState(defaultConfig.target);
  const [newPlaylist, setNewPlaylist] = useState("");
  const [renameValue, setRenameValue] = useState("");
  const [dryRun, setDryRun] = useState<Record<string, unknown> | null>(null);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [busy, setBusy] = useState(false);

  const target = config.target || defaultConfig.target;
  const counts = library?.counts ?? {};
  const queue = library?.queue ?? [];
  const player = library?.player;
  const nowPlaying = player?.track ?? null;
  const connected = library?.status === "ok" || library?.status === "warming_up";
  const selectedPlaylist = playlists.find((playlist) => playlist.id === selectedPlaylistId) ?? null;

  const filteredTracks = useMemo(() => {
    const text = query.trim().toLowerCase();
    const source =
      section === "favorites"
        ? tracks.filter((track) => track.favorited)
        : section === "songs" || section === "library"
          ? tracks
          : tracks;
    if (!text) return source;
    return source.filter((track) =>
      [track.title, track.artist, track.album, track.path].some((value) => value?.toLowerCase().includes(text))
    );
  }, [query, section, tracks]);

  const albums = useMemo(() => groupAlbums(tracks), [tracks]);
  const artists = useMemo(() => groupArtists(tracks), [tracks]);

  useEffect(() => {
    document.documentElement.classList.add("dark");
    document.documentElement.dir = "ltr";
    void boot();
    const light = window.setInterval(() => void refreshLight(), 1400);
    const slower = window.setInterval(() => void refreshLibraryData(false), 4500);
    return () => {
      window.clearInterval(light);
      window.clearInterval(slower);
    };
  }, []);

  async function boot() {
    try {
      const nextConfig = await ipc().getConfig();
      setConfig(nextConfig);
      setTargetDraft(nextConfig.target);
    } catch {
      toast.warning("Electron bridge unavailable", { description: "The UI can still preview, but device actions need Electron." });
    }
    await refreshAll();
  }

  async function refreshAll() {
    await Promise.allSettled([
      refreshLibraryData(true),
      refreshSettings(),
      refreshPlaylists(),
      refreshMixes(),
      refreshSyncState(),
      refreshLogs()
    ]);
  }

  async function refreshLight() {
    await Promise.allSettled([refreshLibraryOnly(), refreshSyncState(), refreshLogs()]);
  }

  async function refreshLibraryOnly() {
    try {
      setLibrary(await getJson<LibraryState>("/api/library"));
    } catch {
      setLibrary(null);
    }
  }

  async function refreshLibraryData(includeTracks = true) {
    await refreshLibraryOnly();
    if (!includeTracks) return;
    try {
      setTracks(await getJson<Track[]>("/api/tracks?limit=500&sort=title"));
    } catch {
      setTracks([]);
    }
  }

  async function refreshPlaylists(nextId = selectedPlaylistId) {
    try {
      const list = await getJson<Playlist[]>("/api/playlists");
      setPlaylists(list);
      const id = nextId && list.some((playlist) => playlist.id === nextId) ? nextId : (list[0]?.id ?? null);
      setSelectedPlaylistId(id);
      if (id) {
        setPlaylistTracks(await getJson<Track[]>(`/api/playlists/${id}/tracks`));
      } else {
        setPlaylistTracks([]);
      }
    } catch {
      setPlaylists([]);
      setPlaylistTracks([]);
    }
  }

  async function refreshMixes() {
    try {
      setMixes(await getJson<Mix[]>("/api/mixes"));
    } catch {
      setMixes([]);
    }
  }

  async function refreshSettings() {
    try {
      setSettings(await getJson<SettingsState>("/api/settings"));
    } catch {
      setSettings({});
    }
  }

  async function refreshSyncState() {
    try {
      const state = await ipc().sync.status();
      setSync(state);
      if (!state.running) setSyncHistory(await ipc().sync.history());
    } catch {
      setSync(null);
    }
  }

  async function refreshLogs() {
    try {
      const [control, runtime] = await Promise.all([
        ipc().emulator.logs(),
        getJson<Array<{ at: number; source: string; line: string }>>("/api/logs?limit=120").catch(() => [])
      ]);
      setConfig((current) => ({ ...current, emulator: control.status ?? current.emulator }));
      setLogs(
        [...(control.logs ?? []), ...runtime.map((line) => ({ ...line, at: new Date(line.at * 1000).toISOString() }))]
          .slice(-180)
          .reverse()
      );
    } catch {}
  }

  async function guarded<T>(label: string, action: () => Promise<T>) {
    setBusy(true);
    const id = toast.loading(label);
    try {
      const result = await action();
      toast.success(label.replace(/\.\.\.$/, " complete"), { id });
      return result;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Action failed", { id });
      throw error;
    } finally {
      setBusy(false);
    }
  }

  async function sendControl(command: string, body: Record<string, unknown> = {}) {
    await postJson("/api/control", { command, ...body });
    await refreshLibraryData(command === "scan_library");
  }

  async function saveTarget() {
    const next = targetDraft.trim();
    if (!next || next === config.target) return;
    const updated = await guarded("Updating target...", () => ipc().setTarget(next));
    setConfig(updated);
    await refreshAll();
  }

  async function saveSettings() {
    await guarded("Saving settings...", async () => {
      await postJson("/api/settings", settings);
      await refreshSettings();
      await refreshLibraryOnly();
    });
  }

  async function createPlaylist() {
    const name = newPlaylist.trim();
    if (!name) return;
    await guarded("Creating playlist...", async () => {
      await postJson("/api/playlists", { name });
      setNewPlaylist("");
      await refreshPlaylists();
    });
  }

  async function renamePlaylist() {
    if (!selectedPlaylist || !renameValue.trim()) return;
    await guarded("Renaming playlist...", async () => {
      await putJson(`/api/playlists/${selectedPlaylist.id}`, { name: renameValue.trim() });
      await refreshPlaylists(selectedPlaylist.id);
    });
  }

  async function deletePlaylist() {
    if (!selectedPlaylist || !window.confirm(`Delete "${selectedPlaylist.name}"?`)) return;
    await guarded("Deleting playlist...", async () => {
      await deleteJson(`/api/playlists/${selectedPlaylist.id}`);
      await refreshPlaylists(null);
    });
  }

  async function addTrackToPlaylist(trackId: number) {
    if (!selectedPlaylistId) {
      toast.info("Create or select a playlist first");
      setSection("playlists");
      return;
    }
    await postJson(`/api/playlists/${selectedPlaylistId}/tracks`, { track_id: trackId });
    await refreshPlaylists(selectedPlaylistId);
    toast.success("Track added to playlist");
  }

  async function startSync() {
    await guarded("Starting sync...", async () => {
      setSync(await ipc().sync.start());
      await refreshSyncState();
      await refreshLogs();
    });
  }

  async function runUsbSync() {
    await guarded("Scanning USB media and updates...", async () => {
      await postJson("/api/sync-usb", {});
      await refreshLibraryOnly();
      await refreshLogs();
    });
  }

  async function installStagedUpdate() {
    if (!window.confirm("Install the staged MusicOS update now?")) return;
    setBusy(true);
    setInstallingUpdate(true);
    const id = toast.loading("Installing staged update...");
    try {
      const result = await postJson<{ status: string; message?: string; installed_update?: string | null }>("/api/updates/install", {});
      await refreshLibraryOnly();
      await refreshLogs();
      if (result.status === "no_update") {
        toast.info(result.message ?? "No staged update was found.", { id });
      } else {
        toast.success(result.message ?? "Installed staged MusicOS update.", {
          id,
          description: result.installed_update ?? undefined
        });
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update install failed", { id });
    } finally {
      setInstallingUpdate(false);
      setBusy(false);
    }
  }

  return (
    <TooltipProvider>
      <div className="font-compact grid h-screen min-h-[720px] grid-cols-[260px_minmax(0,1fr)] overflow-hidden bg-[#101216] text-foreground">
        <aside className="flex min-h-0 flex-col border-r border-white/10 bg-[#171a20]">
          <div className="flex h-16 items-center gap-3 border-b border-white/10 px-4">
            <div className="font-heading grid size-10 place-items-center rounded-lg bg-primary text-lg font-black text-primary-foreground">m</div>
            <div className="min-w-0">
              <div className="font-display truncate text-sm font-semibold">MusicOS Control Center</div>
              <div className="font-micro truncate text-xs text-muted-foreground">iTunes-style companion</div>
            </div>
          </div>
          <ScrollArea className="flex-1">
            <nav className="space-y-5 p-3">
              {["Library", "Playlists", "Device", "System"].map((group) => (
                <div key={group}>
                  <p className="font-micro px-2 pb-2 text-[11px] font-semibold uppercase text-muted-foreground">{group}</p>
                  <div className="grid gap-1">
                    {sections
                      .filter((item) => item.group === group)
                      .map((item) => (
                        <button
                          key={item.id}
                          className={cn(
                            "flex h-9 items-center gap-3 rounded-md px-3 text-sm transition",
                            section === item.id ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-white/7 hover:text-foreground"
                          )}
                          onClick={() => setSection(item.id)}
                        >
                          <item.icon className="size-4" />
                          <span>{item.label}</span>
                        </button>
                      ))}
                  </div>
                </div>
              ))}
            </nav>
          </ScrollArea>
          <div className="border-t border-white/10 p-3">
            <StatusPill connected={connected} emulator={config.emulator} />
          </div>
        </aside>

        <main className="grid min-h-0 grid-rows-[64px_minmax(0,1fr)_96px]">
          <TopBar
            section={section}
            connected={connected}
            query={query}
            setQuery={setQuery}
            scan={() => guarded("Requesting scan...", () => sendControl("scan_library"))}
            refresh={refreshAll}
            target={targetDraft}
            setTarget={setTargetDraft}
            saveTarget={saveTarget}
            busy={busy}
          />

          <ScrollArea className="min-h-0 bg-[#111418]">
            <div className="p-5">
              {section === "library" && (
                <LibraryHome
                  counts={counts}
                  albums={albums}
                  artists={artists}
                  mixes={mixes}
                  tracks={filteredTracks}
                  target={target}
                  playTrack={(id) => sendControl("play_track", { id })}
                  playMix={(id) => sendControl("play_mix", { id, start_index: 0 })}
                  toggleFavorite={(id) => sendControl("toggle_favorite", { id })}
                  addTrack={addTrackToPlaylist}
                />
              )}
              {section === "songs" && (
                <SongsTable
                  tracks={filteredTracks}
                  target={target}
                  playTrack={(id) => sendControl("play_track", { id })}
                  toggleFavorite={(id) => sendControl("toggle_favorite", { id })}
                  queueTrack={(id, position) => sendControl("queue_add", { id, position })}
                  addTrack={addTrackToPlaylist}
                />
              )}
              {section === "albums" && <AlbumGrid albums={albums} target={target} playTrack={(id) => sendControl("play_track", { id })} />}
              {section === "artists" && <ArtistView artists={artists} />}
              {section === "favorites" && (
                <SongsTable
                  tracks={filteredTracks}
                  target={target}
                  playTrack={(id) => sendControl("play_track", { id })}
                  toggleFavorite={(id) => sendControl("toggle_favorite", { id })}
                  queueTrack={(id, position) => sendControl("queue_add", { id, position })}
                  addTrack={addTrackToPlaylist}
                />
              )}
              {section === "playlists" && (
                <PlaylistView
                  playlists={playlists}
                  selected={selectedPlaylist}
                  selectedId={selectedPlaylistId}
                  setSelectedId={(id) => {
                    setSelectedPlaylistId(id);
                    void refreshPlaylists(id);
                  }}
                  tracks={playlistTracks}
                  target={target}
                  newPlaylist={newPlaylist}
                  setNewPlaylist={setNewPlaylist}
                  createPlaylist={createPlaylist}
                  renameValue={renameValue}
                  setRenameValue={setRenameValue}
                  renamePlaylist={renamePlaylist}
                  deletePlaylist={deletePlaylist}
                  playPlaylist={(id) => sendControl("play_playlist", { id, start_index: 0 })}
                  removeTrack={(trackId) => deleteJson(`/api/playlists/${selectedPlaylistId}/tracks/${trackId}`).then(() => refreshPlaylists(selectedPlaylistId))}
                />
              )}
              {section === "devices" && (
                <DevicePanel
                  config={config}
                  library={library}
                  sync={sync}
                  history={syncHistory}
                  dryRun={dryRun}
                  start={() => guarded("Starting emulator...", async () => setConfig({ ...config, emulator: await ipc().emulator.start() }))}
                  stop={() => guarded("Stopping emulator...", async () => setConfig({ ...config, emulator: await ipc().emulator.stop() }))}
                  restart={() => guarded("Restarting emulator...", async () => setConfig({ ...config, emulator: await ipc().emulator.restart() }))}
                  chooseFolder={() => guarded("Choosing folder...", async () => setConfig(await ipc().chooseFolder()))}
                  openFolder={() => guarded("Opening folder...", () => ipc().openFolder())}
                  resetDb={() => {
                    if (window.confirm("Clear the desktop database?")) void guarded("Clearing database...", async () => setConfig(await ipc().resetDb()));
                  }}
                  exportDiagnostics={() => guarded("Exporting diagnostics...", () => ipc().exportDiagnostics())}
                />
              )}
              {section === "sync" && (
                <SyncManager
                  config={config}
                  sync={sync}
                  history={syncHistory}
                  dryRun={dryRun}
                  dryRunAction={() => guarded("Planning sync...", async () => setDryRun(await ipc().sync.dryRun()))}
                  start={startSync}
                  cancel={() => guarded("Cancelling sync...", async () => setSync(await ipc().sync.cancel()))}
                  retry={() => guarded("Retrying failed items...", async () => setSync(await ipc().sync.retryFailed()))}
                  clearHistory={() => guarded("Clearing sync history...", async () => setSyncHistory(await ipc().sync.clearHistory()))}
                />
              )}
              {section === "updates" && (
                <UpdatePanel
                  library={library}
                  runUsbSync={runUsbSync}
                  installUpdate={installStagedUpdate}
                  installingUpdate={installingUpdate}
                  openFolder={() => ipc().openFolder()}
                  logs={logs}
                />
              )}
              {section === "settings" && (
                <SettingsView
                  settings={settings}
                  setSettings={setSettings}
                  save={saveSettings}
                  backup={() => guarded("Backing up library...", () => ipc().backupLibrary())}
                  restore={() => guarded("Restoring library...", () => ipc().restoreLibrary()).then(refreshAll)}
                  exportPlaylists={() => guarded("Exporting playlists...", () => ipc().exportPlaylists())}
                  importPlaylists={() => guarded("Importing playlists...", () => ipc().importPlaylists()).then(() => refreshPlaylists())}
                  openArt={() => guarded("Opening cover art cache...", () => ipc().openCoverArtCache())}
                />
              )}
            </div>
          </ScrollArea>

          <NowPlayingBar
            target={target}
            track={nowPlaying}
            queue={queue}
            player={player}
            control={sendControl}
            volume={settings.volume ?? 0.85}
            setVolume={(value) => setSettings((current) => ({ ...current, volume: value }))}
          />
        </main>
      </div>
      <Toaster position="top-right" richColors />
    </TooltipProvider>
  );

  function getJson<T>(path: string) {
    return fetchJson<T>(target, path);
  }
  function postJson<T = unknown>(path: string, body: unknown) {
    return sendJson<T>(target, "POST", path, body);
  }
  function putJson<T = unknown>(path: string, body: unknown) {
    return sendJson<T>(target, "PUT", path, body);
  }
  function deleteJson<T = unknown>(path: string) {
    return fetch(`${target}${path}`, { method: "DELETE" }).then((response) => {
      if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
      return response.text() as Promise<T>;
    });
  }
}

function TopBar(props: {
  section: Section;
  connected: boolean;
  query: string;
  setQuery: (value: string) => void;
  scan: () => void;
  refresh: () => void;
  target: string;
  setTarget: (value: string) => void;
  saveTarget: () => void;
  busy: boolean;
}) {
  return (
    <header className="flex items-center gap-3 border-b border-white/10 bg-[#1b1f26] px-4">
      <div className="min-w-[150px]">
        <h1 className="font-display text-base font-semibold">{titleFor(props.section)}</h1>
        <p className="font-micro text-xs text-muted-foreground">{props.connected ? "MusicOS API online" : "Waiting for runtime"}</p>
      </div>
      <div className="relative max-w-xl flex-1">
        <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input className="h-9 rounded-md bg-black/20 pl-9" placeholder="Search songs, artists, albums..." value={props.query} onChange={(event) => props.setQuery(event.target.value)} />
      </div>
      <Input
        className="hidden h-9 max-w-[250px] rounded-md bg-black/20 xl:block"
        value={props.target}
        onChange={(event) => props.setTarget(event.target.value)}
        onBlur={props.saveTarget}
        onKeyDown={(event) => {
          if (event.key === "Enter") props.saveTarget();
        }}
      />
      <TooltipButton label="Scan library" icon={RefreshCcw} onClick={props.scan} disabled={props.busy} />
      <TooltipButton label="Refresh" icon={Wand2} onClick={props.refresh} disabled={props.busy} />
    </header>
  );
}

function LibraryHome(props: {
  counts: Record<string, number>;
  albums: AlbumGroup[];
  artists: ArtistGroup[];
  mixes: Mix[];
  tracks: Track[];
  target: string;
  playTrack: (id: number) => void;
  playMix: (id: string) => void;
  toggleFavorite: (id: number) => void;
  addTrack: (id: number) => void;
}) {
  return (
    <div className="grid gap-5">
      <div className="grid gap-3 md:grid-cols-4">
        <Metric label="Songs" value={props.counts.tracks ?? props.tracks.length} />
        <Metric label="Albums" value={props.counts.albums ?? props.albums.length} />
        <Metric label="Artists" value={props.counts.artists ?? props.artists.length} />
        <Metric label="Favorites" value={props.counts.favorites ?? props.tracks.filter((track) => track.favorited).length} />
      </div>
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <Card className="rounded-lg border-white/10 bg-[#181c22]">
          <CardHeader>
            <CardTitle className="text-base">Recently Added</CardTitle>
          </CardHeader>
          <CardContent>
            <SongsTable {...props} tracks={[...props.tracks].sort((a, b) => (b.indexed_at ?? 0) - (a.indexed_at ?? 0)).slice(0, 8)} compact queueTrack={() => {}} />
          </CardContent>
        </Card>
        <Card className="rounded-lg border-white/10 bg-[#181c22]">
          <CardHeader>
            <CardTitle className="text-base">Smart Mixes</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2">
            {props.mixes.length ? (
              props.mixes.slice(0, 8).map((mix) => (
                <button key={mix.id} className="flex items-center justify-between rounded-md border border-white/10 bg-black/15 p-3 text-left hover:bg-white/7" onClick={() => props.playMix(mix.id)}>
                  <span>
                    <span className="block text-sm font-medium">{mix.name}</span>
                    <span className="text-xs text-muted-foreground">{mix.track_count} tracks</span>
                  </span>
                  <Play className="size-4" />
                </button>
              ))
            ) : (
              <EmptyState title="No mixes yet" body="Scan music to generate dynamic mixes." />
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function SongsTable(props: {
  tracks: Track[];
  target: string;
  playTrack: (id: number) => void;
  toggleFavorite: (id: number) => void;
  queueTrack?: (id: number, position?: string) => void;
  addTrack: (id: number) => void;
  compact?: boolean;
}) {
  if (!props.tracks.length) return <EmptyState title="No music here yet" body="Scan or sync music to populate this view. No demo tracks are invented." />;
  return (
    <div className="overflow-hidden rounded-lg border border-white/10">
      <Table>
        {!props.compact && (
          <TableHeader>
            <TableRow className="border-white/10 hover:bg-transparent">
              <TableHead className="w-10">#</TableHead>
              <TableHead>Song</TableHead>
              <TableHead>Artist</TableHead>
              <TableHead>Album</TableHead>
              <TableHead className="w-20">Time</TableHead>
              <TableHead className="w-24 text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
        )}
        <TableBody>
          {props.tracks.map((track, index) => (
            <TableRow key={track.id} className="border-white/10">
              <TableCell className="text-muted-foreground">{track.track_number || index + 1}</TableCell>
              <TableCell>
                <div className="flex min-w-0 items-center gap-3">
                  <CoverArt target={props.target} track={track} />
                  <div className="min-w-0">
                    <button className="block truncate text-left font-medium hover:text-primary" onClick={() => props.playTrack(track.id)}>{track.title || fileTitle(track.path)}</button>
                    <div className="truncate text-xs text-muted-foreground">{track.path}</div>
                  </div>
                </div>
              </TableCell>
              <TableCell className="max-w-[180px] truncate">{track.artist || "Unknown Artist"}</TableCell>
              <TableCell className="max-w-[220px] truncate">{track.album || "Unknown Album"}</TableCell>
              <TableCell>{formatDuration(track.duration)}</TableCell>
              <TableCell className="text-right">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button size="icon-sm" variant="ghost" aria-label={`Actions for ${track.title || fileTitle(track.path)}`}>
                      <ChevronDown className="size-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => props.playTrack(track.id)}>Play</DropdownMenuItem>
                    <DropdownMenuItem onClick={() => props.queueTrack?.(track.id, "next")}>Play next</DropdownMenuItem>
                    <DropdownMenuItem onClick={() => props.addTrack(track.id)}>Add to selected playlist</DropdownMenuItem>
                    <DropdownMenuItem onClick={() => props.toggleFavorite(track.id)}>{track.favorited ? "Unfavorite" : "Favorite"}</DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

type AlbumGroup = { name: string; artist: string; tracks: Track[]; duration: number };
type ArtistGroup = { name: string; albums: number; tracks: number; plays: number };

function AlbumGrid({ albums, target, playTrack }: { albums: AlbumGroup[]; target: string; playTrack: (id: number) => void }) {
  if (!albums.length) return <EmptyState title="No albums indexed" body="Albums appear after metadata scanning finishes." />;
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
      {albums.map((album) => (
        <button key={album.name} className="rounded-lg border border-white/10 bg-[#181c22] p-3 text-left hover:bg-white/7" onClick={() => playTrack(album.tracks[0].id)}>
          <CoverArt target={target} track={album.tracks[0]} className="mb-3 size-full aspect-square rounded-md" />
          <strong className="block truncate text-sm">{album.name}</strong>
          <span className="block truncate text-xs text-muted-foreground">{album.artist} / {album.tracks.length} songs</span>
        </button>
      ))}
    </div>
  );
}

function ArtistView({ artists }: { artists: ArtistGroup[] }) {
  if (!artists.length) return <EmptyState title="No artists indexed" body="Artist metadata will appear after a scan." />;
  return (
    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
      {artists.map((artist) => (
        <Card key={artist.name} className="rounded-lg border-white/10 bg-[#181c22]">
          <CardContent className="flex items-center justify-between p-4">
            <div>
              <h3 className="font-semibold">{artist.name}</h3>
              <p className="text-sm text-muted-foreground">{artist.albums} albums / {artist.tracks} tracks</p>
            </div>
            <Badge variant="secondary">{artist.plays} plays</Badge>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function PlaylistView(props: {
  playlists: Playlist[];
  selected: Playlist | null;
  selectedId: number | null;
  setSelectedId: (id: number) => void;
  tracks: Track[];
  target: string;
  newPlaylist: string;
  setNewPlaylist: (value: string) => void;
  createPlaylist: () => void;
  renameValue: string;
  setRenameValue: (value: string) => void;
  renamePlaylist: () => void;
  deletePlaylist: () => void;
  playPlaylist: (id: number) => void;
  removeTrack: (id: number) => void;
}) {
  return (
    <div className="grid min-h-[560px] gap-4 xl:grid-cols-[300px_minmax(0,1fr)]">
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader>
          <CardTitle className="text-base">Playlists</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="flex gap-2">
            <Input placeholder="New playlist" value={props.newPlaylist} onChange={(event) => props.setNewPlaylist(event.target.value)} onKeyDown={(event) => event.key === "Enter" && props.createPlaylist()} />
            <Button size="icon" onClick={props.createPlaylist}><Plus className="size-4" /></Button>
          </div>
          <Separator />
          {props.playlists.length ? props.playlists.map((playlist) => (
            <button key={playlist.id} className={cn("rounded-md px-3 py-2 text-left text-sm", props.selectedId === playlist.id ? "bg-primary text-primary-foreground" : "hover:bg-white/7")} onClick={() => props.setSelectedId(playlist.id)}>
              {playlist.name}
            </button>
          )) : <EmptyState title="No playlists" body="Create a playlist to start arranging music." />}
        </CardContent>
      </Card>
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-base">{props.selected?.name ?? "Select a playlist"}</CardTitle>
          <div className="flex gap-2">
            <Button variant="secondary" disabled={!props.selected} onClick={() => props.selected && props.playPlaylist(props.selected.id)}><Play className="size-4" /> Play</Button>
            <Dialog onOpenChange={(open) => open && props.setRenameValue(props.selected?.name ?? "")}>
              <DialogTrigger asChild><Button variant="outline" disabled={!props.selected}>Rename</Button></DialogTrigger>
              <DialogContent>
                <DialogHeader><DialogTitle>Rename Playlist</DialogTitle></DialogHeader>
                <Input value={props.renameValue} onChange={(event) => props.setRenameValue(event.target.value)} />
                <DialogFooter><Button onClick={props.renamePlaylist}>Save</Button></DialogFooter>
              </DialogContent>
            </Dialog>
            <Button variant="destructive" disabled={!props.selected} onClick={props.deletePlaylist}><Trash2 className="size-4" /></Button>
          </div>
        </CardHeader>
        <CardContent>
          {props.tracks.length ? (
            <div className="grid gap-2">
              {props.tracks.map((track, index) => (
                <div key={track.id} className="flex items-center gap-3 rounded-md border border-white/10 bg-black/15 p-2">
                  <span className="w-8 text-center text-xs text-muted-foreground">{index + 1}</span>
                  <CoverArt target={props.target} track={track} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{track.title}</div>
                    <div className="truncate text-xs text-muted-foreground">{track.artist} / {track.album}</div>
                  </div>
                  <Button size="icon-sm" variant="ghost" onClick={() => props.removeTrack(track.id)}><Trash2 className="size-4" /></Button>
                </div>
              ))}
            </div>
          ) : <EmptyState title="Playlist is empty" body="Use song menus to add tracks to the selected playlist." />}
        </CardContent>
      </Card>
    </div>
  );
}

function DevicePanel(props: {
  config: Config;
  library: LibraryState | null;
  sync: SyncState | null;
  history: SyncState[];
  dryRun: Record<string, unknown> | null;
  start: () => void;
  stop: () => void;
  restart: () => void;
  chooseFolder: () => void;
  openFolder: () => void;
  resetDb: () => void;
  exportDiagnostics: () => void;
}) {
  const emulator = props.config.emulator;
  const lastSync = props.history[0] ?? null;
  const plannedBytes = Number(props.dryRun?.totalBytes ?? 0);
  const plannedFiles = Number(props.dryRun?.files ?? 0);
  const syncStatus = props.sync?.running ? "Syncing" : lastSync ? (lastSync.failed ? "Needs review" : "Complete") : "Never synced";
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Connected MusicOS Device</CardTitle></CardHeader>
        <CardContent className="grid gap-4">
          <div className="grid gap-3 md:grid-cols-3">
            <Metric label="Runtime" value={emulator?.running ? "Running" : emulator?.starting ? "Starting" : "Stopped"} />
            <Metric label="Tracks" value={props.library?.counts?.tracks ?? 0} />
            <Metric label="Sync status" value={syncStatus} />
          </div>
          <div className="grid gap-3 md:grid-cols-3">
            <Metric label="Storage planned" value={plannedBytes ? formatBytes(plannedBytes) : "Plan needed"} />
            <Metric label="Pending files" value={plannedFiles || "Plan needed"} />
            <Metric label="Last sync" value={lastSync?.finishedAt ? formatDateTime(lastSync.finishedAt) : "Never"} />
          </div>
          <div className="grid gap-2 text-sm text-muted-foreground">
            <PathLine label="Target" value={props.config.target} />
            <PathLine label="Music folder" value={props.config.syncDir} />
            <PathLine label="Database" value={props.config.dbPath} />
            <PathLine label="Binary" value={emulator?.binPath ?? "target/debug/musicos-mac.exe"} />
          </div>
          {lastSync && (
            <div className="rounded-md border border-white/10 bg-black/20 p-3 text-sm">
              <div className="font-medium">Last sync summary</div>
              <div className="mt-1 text-muted-foreground">
                {lastSync.uploaded} uploaded / {lastSync.skipped} skipped / {lastSync.failed} failed
              </div>
            </div>
          )}
          <div className="flex flex-wrap gap-2">
            <Button onClick={props.start}><MonitorPlay className="size-4" /> Start</Button>
            <Button variant="secondary" onClick={props.restart}><RefreshCcw className="size-4" /> Restart</Button>
            <Button variant="outline" onClick={props.stop}><CircleStop className="size-4" /> Stop</Button>
            <Button variant="outline" onClick={props.exportDiagnostics}><BadgeCheck className="size-4" /> Export diagnostics</Button>
          </div>
        </CardContent>
      </Card>
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Device Tools</CardTitle></CardHeader>
        <CardContent className="grid gap-2">
          <Button variant="secondary" onClick={props.chooseFolder}><FolderOpen className="size-4" /> Choose music folder</Button>
          <Button variant="secondary" onClick={props.openFolder}><FolderOpen className="size-4" /> Open music folder</Button>
          <Button variant="outline" onClick={props.resetDb}><Trash2 className="size-4" /> Clear desktop DB</Button>
          <Button variant="outline" disabled><Unplug className="size-4" /> Eject device</Button>
          <p className="text-xs text-muted-foreground">Eject is disabled until a removable-device backend exposes a concrete device id.</p>
        </CardContent>
      </Card>
    </div>
  );
}

function SyncManager(props: {
  config: Config;
  sync: SyncState | null;
  history: SyncState[];
  dryRun: Record<string, unknown> | null;
  dryRunAction: () => void;
  start: () => void;
  cancel: () => void;
  retry: () => void;
  clearHistory: () => void;
}) {
  const state = props.sync;
  const total = state?.found || 0;
  const done = (state?.uploaded || 0) + (state?.skipped || 0) + (state?.failed || 0);
  const dryRunFiles = Number(props.dryRun?.files ?? 0);
  const dryRunBytes = Number(props.dryRun?.totalBytes ?? 0);
  const unreadable = Number(props.dryRun?.unreadable ?? 0);
  const conflicts = [
    ...(state?.failedFiles ?? []).map((file) => `Failed: ${file}`),
    ...(state?.errors ?? [])
  ];
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Sync Music</CardTitle></CardHeader>
        <CardContent className="grid gap-4">
          <div className="grid gap-2 text-sm text-muted-foreground">
            <PathLine label="Source" value={props.config.syncDir} />
            <PathLine label="Target API" value={props.config.target} />
          </div>
          <div className="rounded-md border border-white/10 bg-black/20 p-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="font-medium">What will sync</div>
                <div className="text-sm text-muted-foreground">All supported audio in the selected music folder</div>
              </div>
              <Badge variant="secondary">Folder sync</Badge>
            </div>
          </div>
          <Progress value={total ? (done / total) * 100 : 0} />
          <div className="grid gap-3 md:grid-cols-4">
            <Metric label="Found" value={total} />
            <Metric label="Uploaded" value={state?.uploaded ?? 0} />
            <Metric label="Skipped" value={state?.skipped ?? 0} />
            <Metric label="Failed" value={state?.failed ?? 0} />
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="secondary" onClick={props.dryRunAction}><Cable className="size-4" /> Plan sync</Button>
            <Button disabled={state?.running} onClick={props.start}><RefreshCcw className="size-4" /> Start sync</Button>
            <Button variant="outline" disabled={!state?.running} onClick={props.cancel}>Cancel</Button>
            <Button variant="outline" disabled={!state?.failedFiles?.length} onClick={props.retry}>Retry failed</Button>
          </div>
          {state?.current && <p className="truncate text-sm text-muted-foreground">Current: {state.current}</p>}
          {conflicts.length ? (
            <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3">
              <div className="font-medium text-destructive">Conflicts and failed items</div>
              <div className="mt-2 grid max-h-28 gap-1 overflow-auto text-xs text-muted-foreground">
                {conflicts.slice(0, 8).map((conflict, index) => <div key={`${conflict}-${index}`} className="truncate">{conflict}</div>)}
              </div>
            </div>
          ) : (
            <div className="rounded-md border border-white/10 bg-black/15 p-3 text-sm text-muted-foreground">
              No sync conflicts reported.
            </div>
          )}
        </CardContent>
      </Card>
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Plan And History</CardTitle></CardHeader>
        <CardContent className="grid gap-3">
          {props.dryRun ? (
            <div className="rounded-md border border-white/10 bg-black/20 p-3 text-sm">
              <div className="font-medium">Pending changes</div>
              <div className="mt-1">{dryRunFiles} files / {formatBytes(dryRunBytes)}</div>
              <div className="text-muted-foreground">{unreadable} unreadable</div>
            </div>
          ) : <p className="text-sm text-muted-foreground">Run a dry plan to see pending local files before upload.</p>}
          <Separator />
          <div className="grid gap-2">
            {props.history.length ? props.history.slice(0, 6).map((job, index) => (
              <div key={`${job.startedAt}-${index}`} className="rounded-md border border-white/10 bg-black/15 p-2 text-xs">
                <div>{job.finishedAt ? new Date(job.finishedAt).toLocaleString() : "Running"}</div>
                <div className="text-muted-foreground">{job.uploaded} up / {job.skipped} skip / {job.failed} fail</div>
              </div>
            )) : <EmptyState title="No sync history" body="Completed sync jobs will appear here." />}
          </div>
          <Button variant="outline" onClick={props.clearHistory}>Clear history</Button>
        </CardContent>
      </Card>
    </div>
  );
}

function UpdatePanel({
  library,
  runUsbSync,
  installUpdate,
  installingUpdate,
  openFolder,
  logs
}: {
  library: LibraryState | null;
  runUsbSync: () => void;
  installUpdate: () => void;
  installingUpdate: boolean;
  openFolder: () => void;
  logs: LogLine[];
}) {
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_420px]">
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">USB Updates</CardTitle></CardHeader>
        <CardContent className="grid gap-4">
          <div className="rounded-lg border border-white/10 bg-black/20 p-4">
            <div className="flex items-center gap-3">
              <Usb className="size-5 text-primary" />
              <div>
                <div className="font-medium">USB media and update staging</div>
                <div className="text-sm text-muted-foreground">{library?.usb_status || "No USB status reported yet."}</div>
              </div>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button onClick={runUsbSync}><Usb className="size-4" /> Scan USB / stage update</Button>
            <Button variant="secondary" onClick={openFolder}><FolderOpen className="size-4" /> Open sync folder</Button>
            <Button variant="outline" onClick={installUpdate} disabled={installingUpdate}>
              {installingUpdate ? <Loader2 className="size-4 animate-spin" /> : <Download className="size-4" />}
              Install staged update
            </Button>
          </div>
          <p className="text-sm text-muted-foreground">Scan stages USB payloads, then install applies the staged MusicOS binary or system image through the Rust update backend.</p>
        </CardContent>
      </Card>
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Update Logs</CardTitle></CardHeader>
        <CardContent className="grid max-h-[460px] gap-2 overflow-auto">
          {logs.slice(0, 30).map((log, index) => (
            <div key={index} className="rounded-md bg-black/20 p-2 font-mono text-xs text-muted-foreground">
              <span className="text-primary">{log.source}</span> {log.line}
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function SettingsView(props: {
  settings: SettingsState;
  setSettings: (update: (current: SettingsState) => SettingsState) => void;
  save: () => void;
  backup: () => void;
  restore: () => void;
  exportPlaylists: () => void;
  importPlaylists: () => void;
  openArt: () => void;
}) {
  const update = (patch: SettingsState) => props.setSettings((current) => ({ ...current, ...patch }));
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Preferences</CardTitle></CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-2">
          <Field label="User name"><Input value={props.settings.user_name ?? ""} onChange={(event) => update({ user_name: event.target.value })} /></Field>
          <Field label="Device name"><Input value={props.settings.device_name ?? ""} onChange={(event) => update({ device_name: event.target.value })} /></Field>
          <Field label="Theme"><SelectValueInput value={stringValue(props.settings.theme, "blue")} values={["blue", "green", "red", "purple", "orange"]} onChange={(theme) => update({ theme })} /></Field>
          <Field label="EQ preset"><SelectValueInput value={stringValue(props.settings.eq_preset, "flat")} values={["flat", "bass", "treble", "vocal"]} onChange={(eq_preset) => update({ eq_preset })} /></Field>
          <Field label="Audio backend"><SelectValueInput value={props.settings.audio_backend ?? "auto"} values={["auto", "rodio", "alsa", "none"]} onChange={(audio_backend) => update({ audio_backend })} /></Field>
          <Field label="ALSA device"><Input value={props.settings.alsa_device ?? ""} onChange={(event) => update({ alsa_device: event.target.value })} /></Field>
          <Field label={`Volume ${Math.round((props.settings.volume ?? 0.85) * 100)}%`}><Slider value={[Math.round((props.settings.volume ?? 0.85) * 100)]} onValueChange={([value]) => update({ volume: value / 100 })} /></Field>
          <Field label={`Brightness ${props.settings.brightness ?? 85}%`}><Slider value={[props.settings.brightness ?? 85]} onValueChange={([brightness]) => update({ brightness })} /></Field>
          <Toggle label="Shuffle" checked={Boolean(props.settings.shuffle)} onCheckedChange={(shuffle) => update({ shuffle })} />
          <Toggle label="Resume queue" checked={props.settings.resume_queue_enabled !== false} onCheckedChange={(resume_queue_enabled) => update({ resume_queue_enabled })} />
          <Field label="Loop mode"><SelectValueInput value={stringValue(props.settings.loop_mode, "none")} values={["none", "all", "one"]} onChange={(loop_mode) => update({ loop_mode })} /></Field>
          <Field label="Library sort"><SelectValueInput value={stringValue(props.settings.library_sort, "title")} values={["title", "artist", "album", "recently_added", "play_count"]} onChange={(library_sort) => update({ library_sort })} /></Field>
          <Field label="Library grouping"><SelectValueInput value={stringValue(props.settings.library_group, "none")} values={["none", "artist", "album", "first_letter"]} onChange={(library_group) => update({ library_group })} /></Field>
          <Field label="Motion mode"><SelectValueInput value={stringValue(props.settings.motion_mode, "balanced")} values={["balanced", "battery_saver", "visuals", "reduced"]} onChange={(motion_mode) => update({ motion_mode })} /></Field>
        </CardContent>
      </Card>
      <Card className="rounded-lg border-white/10 bg-[#181c22]">
        <CardHeader><CardTitle className="text-base">Library Tools</CardTitle></CardHeader>
        <CardContent className="grid gap-2">
          <Button onClick={props.save}><SlidersHorizontal className="size-4" /> Save settings</Button>
          <Button variant="secondary" onClick={props.backup}>Back up library</Button>
          <Button variant="secondary" onClick={props.restore}>Restore library</Button>
          <Button variant="outline" onClick={props.exportPlaylists}>Export playlists</Button>
          <Button variant="outline" onClick={props.importPlaylists}>Import playlists</Button>
          <Button variant="outline" onClick={props.openArt}>Open cover-art cache</Button>
        </CardContent>
      </Card>
    </div>
  );
}

function NowPlayingBar({ target, track, queue, player, control, volume, setVolume }: {
  target: string;
  track: Track | null;
  queue: Track[];
  player?: LibraryState["player"];
  control: (command: string, body?: Record<string, unknown>) => void;
  volume: number;
  setVolume: (value: number) => void;
}) {
  const position = player?.position ?? 0;
  const progress = track?.duration ? (position / track.duration) * 100 : 0;
  return (
    <footer className="grid grid-cols-[270px_minmax(0,1fr)_270px] items-center gap-4 border-t border-white/10 bg-[#1b1f26] px-4">
      <div className="flex min-w-0 items-center gap-3">
        {track ? <CoverArt target={target} track={track} /> : <div className="size-12 rounded-md bg-black/30" />}
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{track?.title ?? "Nothing playing"}</div>
          <div className="truncate text-xs text-muted-foreground">{track ? `${track.artist} / ${track.album}` : `${queue.length} queued`}</div>
        </div>
      </div>
      <div className="grid gap-2">
        <div className="flex items-center justify-center gap-2">
          <TooltipButton label="Shuffle" icon={Shuffle} onClick={() => control("shuffle")} />
          <TooltipButton label="Previous" icon={SkipBack} onClick={() => control("prev")} />
          <Button size="icon" onClick={() => control("play_pause")}>{player?.is_playing ? <Pause className="size-4" /> : <Play className="size-4" />}</Button>
          <TooltipButton label="Next" icon={SkipForward} onClick={() => control("next")} />
          <TooltipButton label="Repeat" icon={Repeat2} onClick={() => control("loop")} />
        </div>
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span>{formatDuration(position)}</span>
          <Progress value={progress} className="h-1.5" />
          <span>{formatDuration(track?.duration ?? 0)}</span>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-xs text-muted-foreground">Vol</span>
        <Slider value={[Math.round(volume * 100)]} onValueChange={([value]) => setVolume(value / 100)} onValueCommit={([value]) => control("volume", { value: value / 100 })} />
      </div>
    </footer>
  );
}

function StatusPill({ connected, emulator }: { connected: boolean; emulator?: EmulatorStatus }) {
  return (
    <div className="rounded-lg border border-white/10 bg-black/20 p-3">
      <div className="flex items-center justify-between">
        <span className="font-micro text-xs text-muted-foreground">Runtime</span>
        <Badge variant={connected ? "default" : "secondary"}>{connected ? "Online" : "Offline"}</Badge>
      </div>
      <div className="mt-2 flex items-center justify-between">
        <span className="font-micro text-xs text-muted-foreground">Emulator</span>
        <Badge variant={emulator?.running ? "default" : "outline"}>{emulator?.running ? `PID ${emulator.pid}` : "Stopped"}</Badge>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <Card className="rounded-lg border-white/10 bg-[#181c22]">
      <CardContent className="p-4">
        <div className="font-micro text-xs text-muted-foreground">{label}</div>
        <div className="font-display mt-2 truncate text-2xl font-semibold">{value}</div>
      </CardContent>
    </Card>
  );
}

function CoverArt({ target, track, className }: { target: string; track: Track; className?: string }) {
  const url = `${target}/api/cover-art.png?id=${encodeURIComponent(track.id)}&v=${track.cover_art_updated_at ?? 0}`;
  return (
    <img
      className={cn("size-12 shrink-0 rounded-md bg-gradient-to-br from-primary/40 to-black/30 object-cover", className)}
      src={url}
      alt=""
      onError={(event) => {
        const img = event.currentTarget;
        if (!img.dataset.fallback) {
          img.dataset.fallback = "1";
          img.src = `${target}/api/cover-art.bmp?id=${encodeURIComponent(track.id)}&v=${track.cover_art_updated_at ?? 0}`;
        } else {
          img.style.visibility = "hidden";
        }
      }}
    />
  );
}

function TooltipButton({ label, icon: Icon, ...props }: { label: string; icon: React.ElementType } & React.ComponentProps<typeof Button>) {
  return (
    <Tooltip>
      <TooltipTrigger asChild><Button size="icon" variant="ghost" {...props}><Icon className="size-4" /></Button></TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="grid min-h-[180px] place-items-center rounded-lg border border-dashed border-white/15 bg-black/10 p-6 text-center">
      <div>
        <div className="font-display font-medium">{title}</div>
        <p className="mt-1 max-w-md text-sm text-muted-foreground">{body}</p>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <Label className="grid gap-2 text-sm text-muted-foreground"><span className="font-micro">{label}</span>{children}</Label>;
}

function Toggle({ label, checked, onCheckedChange }: { label: string; checked: boolean; onCheckedChange: (value: boolean) => void }) {
  return <Label className="flex items-center justify-between rounded-md border border-white/10 p-3 text-sm">{label}<Switch checked={checked} onCheckedChange={onCheckedChange} /></Label>;
}

function SelectValueInput({ value, values, onChange }: { value: string; values: string[]; onChange: (value: string) => void }) {
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger><SelectValue /></SelectTrigger>
      <SelectContent>{values.map((item) => <SelectItem key={item} value={item}>{labelize(item)}</SelectItem>)}</SelectContent>
    </Select>
  );
}

function PathLine({ label, value }: { label: string; value?: string }) {
  return <div className="grid grid-cols-[110px_minmax(0,1fr)] gap-3"><span className="font-micro">{label}</span><code className="truncate rounded bg-black/20 px-2 py-1">{value || "-"}</code></div>;
}

function groupAlbums(tracks: Track[]): AlbumGroup[] {
  const map = new Map<string, AlbumGroup>();
  for (const track of tracks) {
    const name = track.album || "Unknown Album";
    const current = map.get(name) ?? { name, artist: track.artist || "Unknown Artist", tracks: [], duration: 0 };
    current.tracks.push(track);
    current.duration += track.duration || 0;
    map.set(name, current);
  }
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
}

function groupArtists(tracks: Track[]): ArtistGroup[] {
  const map = new Map<string, { albums: Set<string>; tracks: number; plays: number }>();
  for (const track of tracks) {
    const name = track.artist || "Unknown Artist";
    const current = map.get(name) ?? { albums: new Set<string>(), tracks: 0, plays: 0 };
    current.albums.add(track.album || "Unknown Album");
    current.tracks += 1;
    current.plays += track.play_count ?? 0;
    map.set(name, current);
  }
  return [...map.entries()].map(([name, value]) => ({ name, albums: value.albums.size, tracks: value.tracks, plays: value.plays })).sort((a, b) => a.name.localeCompare(b.name));
}

function fetchJson<T>(target: string, path: string): Promise<T> {
  return fetch(`${target}${path}`).then((response) => {
    if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
    return response.json() as Promise<T>;
  });
}

function sendJson<T>(target: string, method: string, path: string, body: unknown): Promise<T> {
  return fetch(`${target}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  }).then(async (response) => {
    if (!response.ok) throw new Error(`${path} HTTP ${response.status}`);
    const text = await response.text();
    return (text ? JSON.parse(text) : {}) as T;
  });
}

function ipc() {
  if (!window.musicos) throw new Error("Electron bridge is not available");
  return window.musicos;
}

function titleFor(section: Section) {
  return sections.find((item) => item.id === section)?.label ?? "Library";
}

function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit ? 1 : 0)} ${units[unit]}`;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit"
  });
}

function fileTitle(path: string) {
  return path?.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "Untitled";
}

function labelize(value: string) {
  return value.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function stringValue(value: unknown, fallback: string) {
  return typeof value === "string" ? value.toLowerCase() : fallback;
}
