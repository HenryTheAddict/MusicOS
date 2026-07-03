import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";

const libraryResponse = {
  status: "warming_up",
  counts: { tracks: 0, albums: 0, artists: 0, playlists: 0, favorites: 0 },
  playlists: [],
  player: null,
  queue: [],
  scan_status: "Idle",
  scan: { state: "idle", indexed: 0, discovered: 0 },
  usb_status: "No USB storage found",
  frame_ms: 0,
  frame_timing: { p95_ms: 0, slow_frames: 0 },
  cover_art_cache_count: 0
};

const settingsResponse = {
  user_name: "MusicOS User",
  device_name: "MusicOS",
  theme: "Blue",
  eq_preset: "Flat",
  audio_backend: "auto",
  alsa_device: "",
  volume: 0.85,
  brightness: 85,
  shuffle: false,
  loop_mode: "None",
  library_sort: "Title",
  library_group: "None",
  motion_mode: "Balanced",
  resume_queue_enabled: true
};

const indexedTrack = {
  id: 42,
  path: "C:/Music/Albums/Blue Static/01 Neon Start.wav",
  title: "Neon Start",
  artist: "Blue Static",
  album: "Control Room",
  duration: 185,
  track_number: 1,
  play_count: 2,
  favorited: false,
  indexed_at: 1_778_000_000,
  cover_art_updated_at: 1_778_000_100
};

const secondIndexedTrack = {
  ...indexedTrack,
  id: 43,
  path: "C:/Music/Albums/Blue Static/02 Neon Return.wav",
  title: "Neon Return",
  track_number: 2
};

let libraryData: Record<string, unknown>;
let tracksData: Array<Record<string, unknown>>;
let playlistsData: Array<Record<string, unknown>>;
let playlistTracksData: Array<Record<string, unknown>>;
let runtimeLogsData: Array<Record<string, unknown>>;

describe("MusicOS Control Center shell", () => {
  beforeEach(() => {
    libraryData = { ...libraryResponse };
    tracksData = [];
    playlistsData = [];
    playlistTracksData = [];
    runtimeLogsData = [];
    vi.stubGlobal("fetch", vi.fn(fetchMock));
    vi.stubGlobal("confirm", vi.fn(() => true));
    window.musicos = {
      getConfig: vi.fn(async () => ({
        target: "http://127.0.0.1:18080",
        syncDir: "C:/Music",
        dbPath: "out/desktop-media.sqlite3",
        emulator: { running: false, pid: null }
      })),
      setTarget: vi.fn(),
      chooseFolder: vi.fn(),
      openFolder: vi.fn(),
      openCoverArtCache: vi.fn(),
      resetDb: vi.fn(),
      exportDiagnostics: vi.fn(),
      backupLibrary: vi.fn(),
      restoreLibrary: vi.fn(),
      exportPlaylists: vi.fn(),
      importPlaylists: vi.fn(),
      performance: {
        smoke: vi.fn(async () => ({ ok: true, finishedAt: "2026-07-03T04:00:00.000Z", output: "ok" })),
        report: vi.fn(async () => null)
      },
      sync: {
        start: vi.fn(),
        status: vi.fn(async () => ({ running: false, found: 0, uploaded: 0, skipped: 0, failed: 0 })),
        dryRun: vi.fn(async () => ({ files: 0, totalBytes: 0, unreadable: 0 })),
        retryFailed: vi.fn(),
        cancel: vi.fn(),
        history: vi.fn(async () => []),
        clearHistory: vi.fn(async () => [])
      },
      emulator: {
        start: vi.fn(),
        stop: vi.fn(),
        restart: vi.fn(),
        status: vi.fn(),
        logs: vi.fn(async () => ({ logs: [], status: { running: false, pid: null } })),
        clearLogs: vi.fn()
      }
    };
  });

  it("renders the iTunes-style shell without demo library content", async () => {
    render(<App />);

    expect(await screen.findByText("MusicOS Control Center")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Songs" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Devices" })).toBeInTheDocument();
    expect(await screen.findByText("No music here yet")).toBeInTheDocument();
    expect(screen.queryByText("Fake Demo Song")).not.toBeInTheDocument();
  });

  it("shows device status safely when no device is connected", async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Devices" }));

    expect(await screen.findByText("Connected MusicOS Device")).toBeInTheDocument();
    expect(screen.getAllByText("Stopped").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /Eject device/i })).toBeDisabled();
  });

  it("renders sync planning and update staging states", async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Sync" }));
    expect(await screen.findByText("Sync Music")).toBeInTheDocument();
    expect(screen.getByText("Run a dry plan to see pending local files before upload.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Updates" }));
    expect(await screen.findByText("USB Updates")).toBeInTheDocument();
    expect(screen.getByText("No USB storage found")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Install staged update/i }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/updates/install",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  it("renders logs, diagnostics, and performance actions", async () => {
    const musicos = window.musicos!;
    musicos.emulator.logs = vi.fn(async () => ({
      logs: [{ at: "2026-07-03T04:00:00.000Z", source: "emulator", line: "boot ready" }],
      status: { running: true, pid: 1234 }
    }));
    musicos.emulator.clearLogs = vi.fn(async () => ({ logs: [], status: { running: true, pid: 1234 } }));
    musicos.performance.report = vi.fn(async () => ({
      ok: false,
      finishedAt: "2026-07-03T04:01:00.000Z",
      error: "frame budget exceeded"
    }));
    runtimeLogsData = [{ at: 1_783_050_000, source: "runtime", line: "scan complete" }];
    libraryData = {
      ...libraryResponse,
      frame_timing: { p95_ms: 18.4, slow_frames: 1 },
      cover_art_cache_count: 12,
      audio: { backend: "rodio" },
      scan_status: "Idle"
    };

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Logs" }));
    expect(await screen.findByText("Control Center Logs")).toBeInTheDocument();
    expect(screen.getByText("boot ready")).toBeInTheDocument();
    expect(screen.getByText("scan complete")).toBeInTheDocument();
    expect(screen.getByText("18.4 ms")).toBeInTheDocument();
    expect(screen.getByText("Last run failed")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Clear control logs/i }));
    fireEvent.click(screen.getByRole("button", { name: /Run smoke/i }));
    fireEvent.click(screen.getByRole("button", { name: /Export diagnostics/i }));

    await waitFor(() => {
      expect(musicos.emulator.clearLogs).toHaveBeenCalled();
      expect(musicos.performance.smoke).toHaveBeenCalled();
      expect(musicos.exportDiagnostics).toHaveBeenCalled();
    });
  });

  it("renders persisted settings and saves through the backend API", async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
    expect(await screen.findByDisplayValue("MusicOS User")).toBeInTheDocument();

    fireEvent.change(screen.getByDisplayValue("MusicOS User"), { target: { value: "Henry" } });
    fireEvent.click(screen.getByRole("button", { name: /Save settings/i }));

    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/settings",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  it("wires song menu actions to player, queue, favorite, and playlist APIs", async () => {
    tracksData = [indexedTrack, secondIndexedTrack];
    playlistsData = [{ id: 7, name: "Pocket Sync", track_count: 0 }];
    playlistTracksData = [indexedTrack, secondIndexedTrack];
    libraryData = {
      ...libraryResponse,
      counts: { tracks: 2, albums: 1, artists: 1, playlists: 1, favorites: 0 },
      player: { track: indexedTrack, is_playing: false, position: 11 },
      queue: [indexedTrack, secondIndexedTrack],
      queue_index: 0
    };

    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Songs" }));
    expect((await screen.findAllByText("Neon Start")).length).toBeGreaterThan(0);

    fireEvent.pointerDown(screen.getByRole("button", { name: /Actions for Neon Start/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Add to selected playlist" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/playlists/7/tracks",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ track_id: 42 })
        })
      );
    });

    fireEvent.pointerDown(screen.getByRole("button", { name: /Actions for Neon Start/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Favorite" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "toggle_favorite", id: 42 })
        })
      );
    });

    fireEvent.pointerDown(screen.getByRole("button", { name: /Actions for Neon Start/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Play next" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "queue_add", id: 42, position: "next" })
        })
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "Neon Start" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "play_track", id: 42 })
        })
      );
    });

    fireEvent.click(screen.getByRole("button", { name: /Open queue with 2 tracks/i }));
    expect(await screen.findByText("Up Next")).toBeInTheDocument();
    expect(screen.getAllByText("Neon Return").length).toBeGreaterThan(0);
    fireEvent.click(screen.getAllByRole("button", { name: "Remove" })[1]);
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "queue_remove", index: 1 })
        })
      );
    });
    fireEvent.click(screen.getByRole("button", { name: "Clear queue" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "queue_clear" })
        })
      );
    });
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    fireEvent.click(screen.getByRole("button", { name: "Albums" }));
    fireEvent.pointerDown(await screen.findByRole("button", { name: /Actions for album Control Room/i }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Add album to selected playlist" }));

    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/playlists/7/tracks",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ track_id: 43 })
        })
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "Flow" }));
    expect((await screen.findAllByText("Flow")).length).toBeGreaterThan(0);
    expect(screen.getByText("Control Room")).toBeInTheDocument();

    const fetchSpy = vi.mocked(fetch);
    fetchSpy.mockClear();
    fireEvent.click(screen.getByRole("button", { name: /Play Flow album/i }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/control",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ command: "play_track", id: 42 })
        })
      );
    });

    fetchSpy.mockClear();
    fireEvent.click(screen.getByRole("button", { name: /Add to playlist/i }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/playlists/7/tracks",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({ track_id: 43 })
        })
      );
    });

    fetchSpy.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Playlists" }));
    expect((await screen.findAllByText("Pocket Sync")).length).toBeGreaterThan(0);
    fireEvent.click(await screen.findByRole("button", { name: "Move Neon Return up" }));
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "http://127.0.0.1:18080/api/playlists/7/tracks/reorder",
        expect.objectContaining({
          method: "PUT",
          body: JSON.stringify({ track_ids: [43, 42] })
        })
      );
    });
  });

  it("wires device emulator and sync controls through Electron IPC", async () => {
    const musicos = window.musicos!;
    tracksData = [indexedTrack];
    playlistsData = [{ id: 7, name: "Pocket Sync", track_count: 1 }];
    playlistTracksData = [indexedTrack];
    musicos.sync.status = vi.fn(async () => ({
      running: false,
      found: 4,
      uploaded: 2,
      skipped: 1,
      failed: 1,
      errors: ["scan: HTTP 500"],
      failedFiles: ["C:/Music/broken.flac"]
    }));
    musicos.sync.dryRun = vi.fn(async () => ({ files: 4, totalBytes: 4096, unreadable: 1 }));
    musicos.sync.history = vi.fn(async () => [{
      running: false,
      found: 4,
      uploaded: 2,
      skipped: 1,
      failed: 1,
      finishedAt: "2026-07-02T18:00:00.000Z"
    }]);
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Songs" }));
    fireEvent.click(await screen.findByRole("checkbox", { name: "Select Neon Start" }));

    fireEvent.click(await screen.findByRole("button", { name: "Devices" }));
    expect(await screen.findByText("Storage planned")).toBeInTheDocument();
    expect(screen.getByText("Needs review")).toBeInTheDocument();
    expect(screen.getByText("Last sync summary")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: /^Start$/i }));
    fireEvent.click(screen.getByRole("button", { name: /^Restart$/i }));
    fireEvent.click(screen.getByRole("button", { name: /^Stop$/i }));

    await waitFor(() => {
      expect(musicos.emulator.start).toHaveBeenCalled();
      expect(musicos.emulator.restart).toHaveBeenCalled();
      expect(musicos.emulator.stop).toHaveBeenCalled();
    });

    fireEvent.click(screen.getByRole("button", { name: "Sync" }));
    expect(await screen.findByText("What will sync")).toBeInTheDocument();
    expect(screen.getByText("All music")).toBeInTheDocument();
    expect(screen.getByText("Conflicts and failed items")).toBeInTheDocument();
    expect(screen.getByText("Failed: C:/Music/broken.flac")).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: /Plan sync/i }));
    expect(await screen.findByText("Pending changes")).toBeInTheDocument();
    expect(screen.getByText("4 files / 4.0 KB")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Start sync/i }));

    await waitFor(() => {
      expect(musicos.sync.dryRun).toHaveBeenCalled();
      expect(musicos.sync.start).toHaveBeenCalled();
    });

    const scopeSelect = screen.getByRole("combobox", { name: "Sync scope" });
    fireEvent.pointerDown(scopeSelect, { button: 0, ctrlKey: false, pointerType: "mouse" });
    fireEvent.click(await screen.findByRole("option", { name: "Selected tracks (1)" }));
    expect(await screen.findByText("1 selected track")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Plan sync/i }));
    await waitFor(() => {
      expect(musicos.sync.dryRun).toHaveBeenLastCalledWith(["C:/Music/Albums/Blue Static/01 Neon Start.wav"]);
    });

    fireEvent.pointerDown(scopeSelect, { button: 0, ctrlKey: false, pointerType: "mouse" });
    fireEvent.click(await screen.findByRole("option", { name: "Selected playlist" }));
    expect(await screen.findByText("Pocket Sync")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Plan sync/i }));
    await waitFor(() => {
      expect(musicos.sync.dryRun).toHaveBeenLastCalledWith(["C:/Music/Albums/Blue Static/01 Neon Start.wav"]);
    });
  });
});

function fetchMock(input: RequestInfo | URL, init?: RequestInit) {
  const url = String(input);
  if (init?.method === "POST" || init?.method === "PUT" || init?.method === "DELETE") {
    return jsonResponse({ status: "ok" });
  }
  if (url.includes("/api/library")) return jsonResponse(libraryData);
  if (url.includes("/api/tracks")) return jsonResponse(tracksData);
  if (url.includes("/api/settings")) return jsonResponse(settingsResponse);
  if (url.includes("/api/playlists/") && url.includes("/tracks")) return jsonResponse(playlistTracksData);
  if (url.includes("/api/playlists")) return jsonResponse(playlistsData);
  if (url.includes("/api/mixes")) return jsonResponse([]);
  if (url.includes("/api/logs")) return jsonResponse(runtimeLogsData);
  return jsonResponse({});
}

function jsonResponse(body: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { "Content-Type": "application/json" }
    })
  );
}
