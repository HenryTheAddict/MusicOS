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

describe("MusicOS Control Center shell", () => {
  beforeEach(() => {
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
    expect(screen.queryByText("Demo Track")).not.toBeInTheDocument();
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
});

function fetchMock(input: RequestInfo | URL, init?: RequestInit) {
  const url = String(input);
  if (init?.method === "POST" || init?.method === "PUT" || init?.method === "DELETE") {
    return jsonResponse({ status: "ok" });
  }
  if (url.includes("/api/library")) return jsonResponse(libraryResponse);
  if (url.includes("/api/tracks")) return jsonResponse([]);
  if (url.includes("/api/settings")) return jsonResponse(settingsResponse);
  if (url.includes("/api/playlists")) return jsonResponse([]);
  if (url.includes("/api/mixes")) return jsonResponse([]);
  if (url.includes("/api/logs")) return jsonResponse([]);
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
