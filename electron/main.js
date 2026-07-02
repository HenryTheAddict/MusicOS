const { app, BrowserWindow, dialog, ipcMain, shell } = require("electron");
const { spawn } = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs/promises");
const fsSync = require("node:fs");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const DEFAULT_PORT = process.env.MUSICOS_COMPANION_PORT || "18080";
const DEFAULT_TARGET = process.env.MUSICOS_SYNC_TARGET || `http://127.0.0.1:${DEFAULT_PORT}`;
const DEFAULT_DIR = process.env.MUSICOS_SYNC_DIR || path.resolve(projectRoot, "music");
const DEFAULT_DB = process.env.MUSICOS_MEDIA_DB || path.resolve(projectRoot, "out", "desktop-media.sqlite3");
const SYNC_HISTORY_PATH = path.resolve(projectRoot, "out", "control-center-sync-history.json");
const MAX_LOG_LINES = 700;
const MAX_SYNC_HISTORY = 50;

let mainWindow;
let syncDir = DEFAULT_DIR;
let target = DEFAULT_TARGET;
let dbPath = DEFAULT_DB;
let emulatorChild = null;
let lastExit = null;
let logs = [];
let syncHistory = null;
let performanceReport = null;
let starting = false;
let syncState = {
  running: false,
  cancelRequested: false,
  found: 0,
  uploaded: 0,
  skipped: 0,
  failed: 0,
  current: "",
  errors: [],
  failedFiles: [],
  startedAt: null,
  finishedAt: null
};

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 1100,
    minHeight: 740,
    title: "MusicOS Control Center",
    backgroundColor: "#e8eef4",
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "default",
    trafficLightPosition: { x: 18, y: 18 },
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  mainWindow.loadFile(path.join(__dirname, "renderer", "index.html"));
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
  stopEmulator();
  if (process.platform !== "darwin") app.quit();
});

app.on("before-quit", () => {
  stopEmulator();
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

ipcMain.handle("config:get", () => config());

ipcMain.handle("config:set-target", (_event, nextTarget) => {
  if (typeof nextTarget === "string" && nextTarget.trim()) {
    target = nextTarget.trim().replace(/\/$/, "");
  }
  return config();
});

ipcMain.handle("folder:choose", async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: "Choose MusicOS sync folder",
    defaultPath: syncDir,
    properties: ["openDirectory", "createDirectory"]
  });
  if (!result.canceled && result.filePaths[0]) {
    syncDir = result.filePaths[0];
  }
  return config();
});

ipcMain.handle("folder:open", async () => {
  await fs.mkdir(syncDir, { recursive: true });
  const error = await shell.openPath(syncDir);
  return { ok: !error, error, ...config() };
});

ipcMain.handle("folder:upload", startSync);
ipcMain.handle("sync:start", startSync);
ipcMain.handle("sync:status", () => syncState);
ipcMain.handle("sync:dryRun", syncDryRun);
ipcMain.handle("sync:retryFailed", retryFailedSync);
ipcMain.handle("sync:history:get", async () => loadSyncHistory());
ipcMain.handle("sync:history:clear", async () => {
  syncHistory = [];
  await saveSyncHistory();
  appendLog("sync", "Sync history cleared");
  return syncHistory;
});
ipcMain.handle("sync:cancel", () => {
  syncState.cancelRequested = true;
  appendLog("sync", "Cancel requested");
  return syncState;
});
ipcMain.handle("diagnostics:export", exportDiagnostics);
ipcMain.handle("library:backup", backupLibrary);
ipcMain.handle("library:restore", restoreLibrary);
ipcMain.handle("library:playlistExport", exportPlaylists);
ipcMain.handle("library:playlistImport", importPlaylists);
ipcMain.handle("folder:openCoverArtCache", openCoverArtCache);
ipcMain.handle("performance:smoke", runPerformanceSmoke);
ipcMain.handle("performance:report:get", () => performanceReport);

ipcMain.handle("emulator:start", startEmulator);
ipcMain.handle("emulator:stop", stopEmulator);
ipcMain.handle("emulator:restart", async () => {
  await stopEmulator();
  return startEmulator();
});
ipcMain.handle("emulator:status", () => emulatorStatus());
ipcMain.handle("emulator:logs:get", () => ({ logs, status: emulatorStatus() }));
ipcMain.handle("emulator:logs:clear", () => {
  logs = [];
  return { logs, status: emulatorStatus() };
});
ipcMain.handle("library:resetDb", async () => {
  await stopEmulator();
  for (const suffix of ["", "-shm", "-wal"]) {
    try {
      await fs.unlink(`${dbPath}${suffix}`);
      appendLog("system", `Deleted ${path.basename(dbPath)}${suffix}`);
    } catch (error) {
      if (error.code !== "ENOENT") appendLog("system", `Reset skipped ${suffix}: ${error.message}`);
    }
  }
  return { ok: true, ...config(), status: emulatorStatus() };
});

function config() {
  return {
    target,
    syncDir,
    dbPath,
    port: DEFAULT_PORT,
    emulator: emulatorStatus()
  };
}

async function startEmulator() {
  if (emulatorChild) return emulatorStatus();
  if (starting) return { ...emulatorStatus(), starting: true };

  starting = true;
  try {
    await fs.mkdir(path.dirname(dbPath), { recursive: true });
    await fs.mkdir(syncDir, { recursive: true });
    const bin = await ensureDesktopBinary();
    appendLog("system", `Starting ${bin}`);

    emulatorChild = spawn(bin, [], {
      cwd: projectRoot,
      env: {
        ...process.env,
        MUSICOS_COMPANION_PORT: DEFAULT_PORT,
        MUSICOS_MEDIA_DB: dbPath,
        MUSICOS_UPLOAD_DIR: syncDir,
        MUSICOS_SYNC_DIR: syncDir,
        MUSICOS_SYNC_TARGET: target
      },
      windowsHide: false
    });

    emulatorChild.stdout.on("data", data => appendLog("emulator", data));
    emulatorChild.stderr.on("data", data => appendLog("emulator", data));
    emulatorChild.on("error", error => {
      appendLog("emulator", `error: ${error.message}`);
      lastExit = { code: null, signal: "error", at: new Date().toISOString() };
      emulatorChild = null;
    });
    emulatorChild.on("exit", (code, signal) => {
      appendLog("emulator", `exited code=${code ?? "null"} signal=${signal ?? "null"}`);
      lastExit = { code, signal, at: new Date().toISOString() };
      emulatorChild = null;
    });

    lastExit = null;
    return emulatorStatus();
  } finally {
    starting = false;
  }
}

async function startSync(_event, filesOverride = null) {
  if (syncState.running) return syncState;

  syncState = {
    running: true,
    cancelRequested: false,
    found: 0,
    uploaded: 0,
    skipped: 0,
    failed: 0,
    current: "",
    errors: [],
    failedFiles: [],
    startedAt: new Date().toISOString(),
    finishedAt: null
  };

  runSyncJob(filesOverride).catch(error => {
    syncState.failed += 1;
    syncState.errors.push(error.message);
    appendLog("sync", `failed: ${error.message}`);
  }).finally(() => {
    syncState.running = false;
    syncState.finishedAt = new Date().toISOString();
    recordSyncHistory(syncState).catch(error => appendLog("sync", `history failed: ${error.message}`));
  });

  return syncState;
}

async function retryFailedSync() {
  const failedFiles = Array.isArray(syncState.failedFiles) ? syncState.failedFiles.slice() : [];
  if (!failedFiles.length) return { ...syncState, retryStarted: false };
  const state = await startSync(null, failedFiles);
  return { ...state, retryStarted: true };
}

async function syncDryRun() {
  const files = await collectAudioFiles(syncDir);
  let totalBytes = 0;
  let unreadable = 0;
  for (const file of files) {
    try {
      const stats = await fs.stat(file);
      totalBytes += stats.size;
    } catch {
      unreadable += 1;
    }
  }
  const result = {
    ok: true,
    syncDir,
    target,
    files: files.length,
    totalBytes,
    unreadable,
    generatedAt: new Date().toISOString()
  };
  appendLog("sync", `Dry run files=${result.files} bytes=${result.totalBytes} unreadable=${unreadable}`);
  return result;
}

async function runSyncJob(filesOverride = null) {
  const files = Array.isArray(filesOverride) ? filesOverride : await collectAudioFiles(syncDir);
  syncState.found = files.length;
  appendLog("sync", `Found ${files.length} audio files in ${syncDir}`);

  for (const file of files) {
    if (syncState.cancelRequested) {
      appendLog("sync", "Sync cancelled");
      break;
    }

    const relativePath = path.relative(syncDir, file);
    syncState.current = relativePath;
    try {
      const body = await fs.readFile(file);
      const stats = await fs.stat(file);
      const sha256 = crypto.createHash("sha256").update(body).digest("hex");
      const response = await fetch(`${target}/api/upload`, {
        method: "POST",
        headers: {
          "X-Filename": path.basename(file),
          "X-Relative-Path": relativePath,
          "X-Sha256": sha256,
          "X-File-Size": String(stats.size),
          "X-Modified-At": String(Math.floor(stats.mtimeMs)),
          "X-Trigger-Scan": "0"
        },
        body
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const result = await response.json().catch(() => ({ status: "ok" }));
      if (result.status === "skipped") {
        syncState.skipped += 1;
      } else {
        syncState.uploaded += 1;
      }
    } catch (error) {
      syncState.failed += 1;
      syncState.failedFiles.push(file);
      syncState.errors.push(`${relativePath}: ${error.message}`);
      appendLog("sync", `${relativePath}: ${error.message}`);
    }
  }

  syncState.current = "";
  if (!syncState.cancelRequested) {
    try {
      await fetch(`${target}/api/control`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ command: "scan_library" })
      });
      appendLog(
        "sync",
        `Complete uploaded=${syncState.uploaded} skipped=${syncState.skipped} failed=${syncState.failed}`
      );
    } catch (error) {
      syncState.failed += 1;
      syncState.errors.push(`scan: ${error.message}`);
      appendLog("sync", `scan trigger failed: ${error.message}`);
    }
  }
}

async function exportDiagnostics() {
  const defaultPath = path.join(
    projectRoot,
    "out",
    `musicos-diagnostics-${new Date().toISOString().replace(/[:.]/g, "-")}.json`
  );
  const result = await dialog.showSaveDialog(mainWindow, {
    title: "Export MusicOS diagnostics",
    defaultPath,
    filters: [{ name: "JSON", extensions: ["json"] }]
  });
  if (result.canceled || !result.filePath) {
    return { ok: false, canceled: true };
  }

  const [diagnostics, library, runtimeLogs] = await Promise.all([
    fetchJson("/api/diagnostics"),
    fetchJson("/api/library"),
    fetchJson("/api/logs?limit=200").catch(() => [])
  ]);
  const payload = {
    exportedAt: new Date().toISOString(),
    target,
    syncDir,
    dbPath,
    emulator: emulatorStatus(),
    diagnostics,
    library,
    runtimeLogs,
    controlCenterLogs: logs,
    sync: syncState
  };
  await fs.mkdir(path.dirname(result.filePath), { recursive: true });
  await fs.writeFile(result.filePath, JSON.stringify(payload, null, 2));
  appendLog("system", `Exported diagnostics to ${result.filePath}`);
  return { ok: true, path: result.filePath };
}

async function backupLibrary() {
  const defaultPath = path.join(
    projectRoot,
    "out",
    `musicos-library-backup-${new Date().toISOString().replace(/[:.]/g, "-")}.json`
  );
  const result = await dialog.showSaveDialog(mainWindow, {
    title: "Back up MusicOS library settings",
    defaultPath,
    filters: [{ name: "JSON", extensions: ["json"] }]
  });
  if (result.canceled || !result.filePath) return { ok: false, canceled: true };

  const [settings, playlists, diagnostics, history] = await Promise.all([
    fetchJson("/api/settings"),
    fetchJson("/api/playlists"),
    fetchJson("/api/diagnostics").catch(() => null),
    fetchJson("/api/history?limit=500").catch(() => [])
  ]);
  const payload = {
    kind: "musicos-library-backup",
    version: 1,
    exportedAt: new Date().toISOString(),
    target,
    syncDir,
    dbPath,
    settings,
    playlists,
    diagnostics,
    history
  };
  await fs.mkdir(path.dirname(result.filePath), { recursive: true });
  await fs.writeFile(result.filePath, JSON.stringify(payload, null, 2));
  appendLog("system", `Library backup saved to ${result.filePath}`);
  return { ok: true, path: result.filePath };
}

async function restoreLibrary() {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: "Restore MusicOS library settings",
    filters: [{ name: "JSON", extensions: ["json"] }],
    properties: ["openFile"]
  });
  if (result.canceled || !result.filePaths[0]) return { ok: false, canceled: true };
  const filePath = result.filePaths[0];
  const payload = JSON.parse(await fs.readFile(filePath, "utf8"));
  let restoredSettings = false;
  let restoredPlaylists = 0;

  if (payload.settings && typeof payload.settings === "object") {
    await fetch(`${target}/api/settings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload.settings)
    });
    restoredSettings = true;
  }

  if (Array.isArray(payload.playlists)) {
    for (const playlist of payload.playlists) {
      if (!playlist?.name) continue;
      await fetch(`${target}/api/playlists`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: playlist.name })
      });
      restoredPlaylists += 1;
    }
  }

  appendLog("system", `Library restore loaded ${filePath}`);
  return { ok: true, restoredSettings, restoredPlaylists, path: filePath };
}

async function exportPlaylists() {
  const defaultPath = path.join(
    projectRoot,
    "out",
    `musicos-playlists-${new Date().toISOString().replace(/[:.]/g, "-")}.json`
  );
  const result = await dialog.showSaveDialog(mainWindow, {
    title: "Export MusicOS playlists",
    defaultPath,
    filters: [{ name: "JSON", extensions: ["json"] }]
  });
  if (result.canceled || !result.filePath) return { ok: false, canceled: true };
  const playlists = await fetchJson("/api/playlists");
  const withTracks = [];
  for (const playlist of playlists) {
    const tracks = await fetchJson(`/api/playlists/${playlist.id}/tracks`).catch(() => []);
    withTracks.push({ ...playlist, tracks });
  }
  await fs.mkdir(path.dirname(result.filePath), { recursive: true });
  await fs.writeFile(result.filePath, JSON.stringify({ kind: "musicos-playlists", playlists: withTracks }, null, 2));
  appendLog("system", `Playlists exported to ${result.filePath}`);
  return { ok: true, path: result.filePath, count: withTracks.length };
}

async function importPlaylists() {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: "Import MusicOS playlists",
    filters: [{ name: "JSON", extensions: ["json"] }],
    properties: ["openFile"]
  });
  if (result.canceled || !result.filePaths[0]) return { ok: false, canceled: true };
  const payload = JSON.parse(await fs.readFile(result.filePaths[0], "utf8"));
  const playlists = Array.isArray(payload.playlists) ? payload.playlists : [];
  let imported = 0;
  for (const playlist of playlists) {
    if (!playlist?.name) continue;
    await fetch(`${target}/api/playlists`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: playlist.name })
    });
    imported += 1;
  }
  appendLog("system", `Imported ${imported} playlists`);
  return { ok: true, imported };
}

async function openCoverArtCache() {
  const diagnostics = await fetchJson("/api/diagnostics").catch(() => ({ db_path: dbPath }));
  const coverArtDir = path.join(path.dirname(diagnostics.db_path || dbPath), "cover-art");
  await fs.mkdir(coverArtDir, { recursive: true });
  const error = await shell.openPath(coverArtDir);
  return { ok: !error, error, path: coverArtDir };
}

async function runPerformanceSmoke() {
  const script = path.join(projectRoot, "scripts", "perf-smoke.ps1");
  const startedAt = new Date().toISOString();
  appendLog("perf", "Starting performance smoke test");
  try {
    await stopEmulator();
    const output = await runCommandCapture("powershell.exe", [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      script,
      "-TrackCount",
      "32",
      "-Samples",
      "20",
      "-TimeoutSeconds",
      "45",
      "-SkipBuild"
    ]);
    performanceReport = { ok: true, startedAt, finishedAt: new Date().toISOString(), output };
    appendLog("perf", "Performance smoke passed");
  } catch (error) {
    performanceReport = {
      ok: false,
      startedAt,
      finishedAt: new Date().toISOString(),
      output: error.output || "",
      error: error.message
    };
    appendLog("perf", `Performance smoke failed: ${error.message}`);
  }
  return performanceReport;
}

async function fetchJson(apiPath) {
  const response = await fetch(`${target}${apiPath}`);
  if (!response.ok) throw new Error(`${apiPath} HTTP ${response.status}`);
  return response.json();
}

async function stopEmulator() {
  const child = emulatorChild;
  if (!child) return emulatorStatus();
  appendLog("system", "Stopping emulator");
  emulatorChild = null;
  child.kill();
  await new Promise(resolve => setTimeout(resolve, 250));
  return emulatorStatus();
}

async function ensureDesktopBinary() {
  const bin = desktopBinaryPath();
  if (fsSync.existsSync(bin)) return bin;

  appendLog("build", "musicos-mac binary missing; building desktop emulator");
  await runCommand(cargoCommand(), [
    "build",
    "--bin",
    "musicos-mac",
    "--features",
    "desktop-app,desktop-audio"
  ]);
  if (!fsSync.existsSync(bin)) {
    throw new Error(`Built emulator binary was not found at ${bin}`);
  }
  return bin;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: projectRoot,
      env: process.env,
      windowsHide: true
    });
    child.stdout.on("data", data => appendLog("build", data));
    child.stderr.on("data", data => appendLog("build", data));
    child.on("error", reject);
    child.on("exit", code => {
      if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(" ")} exited with ${code}`));
    });
  });
}

function runCommandCapture(command, args) {
  return new Promise((resolve, reject) => {
    let output = "";
    const child = spawn(command, args, {
      cwd: projectRoot,
      env: process.env,
      windowsHide: true
    });
    child.stdout.on("data", data => {
      output += data.toString("utf8");
      appendLog("perf", data);
    });
    child.stderr.on("data", data => {
      output += data.toString("utf8");
      appendLog("perf", data);
    });
    child.on("error", error => {
      error.output = output;
      reject(error);
    });
    child.on("exit", code => {
      if (code === 0) resolve(output);
      else {
        const error = new Error(`${command} ${args.join(" ")} exited with ${code}`);
        error.output = output;
        reject(error);
      }
    });
  });
}

function emulatorStatus() {
  return {
    running: Boolean(emulatorChild),
    starting,
    pid: emulatorChild?.pid || null,
    target,
    dbPath,
    syncDir,
    binPath: desktopBinaryPath(),
    lastExit
  };
}

function desktopBinaryPath() {
  const targetDir = process.env.CARGO_TARGET_DIR || path.resolve(projectRoot, "target");
  const profile = process.env.MUSICOS_DESKTOP_PROFILE || "debug";
  const binary = process.platform === "win32" ? "musicos-mac.exe" : "musicos-mac";
  return path.join(targetDir, profile, binary);
}

function cargoCommand() {
  return process.platform === "win32" ? "cargo.exe" : "cargo";
}

function appendLog(source, data) {
  const text = Buffer.isBuffer(data) ? data.toString("utf8") : String(data);
  for (const line of text.split(/\r?\n/)) {
    if (!line.trim()) continue;
    logs.push({
      at: new Date().toISOString(),
      source,
      line
    });
  }
  if (logs.length > MAX_LOG_LINES) {
    logs = logs.slice(logs.length - MAX_LOG_LINES);
  }
}

async function loadSyncHistory() {
  if (syncHistory) return syncHistory;
  try {
    const parsed = JSON.parse(await fs.readFile(SYNC_HISTORY_PATH, "utf8"));
    syncHistory = Array.isArray(parsed) ? parsed : [];
  } catch {
    syncHistory = [];
  }
  return syncHistory;
}

async function saveSyncHistory() {
  await fs.mkdir(path.dirname(SYNC_HISTORY_PATH), { recursive: true });
  await fs.writeFile(SYNC_HISTORY_PATH, JSON.stringify(syncHistory || [], null, 2));
}

async function recordSyncHistory(state) {
  const history = await loadSyncHistory();
  history.unshift({
    startedAt: state.startedAt,
    finishedAt: state.finishedAt,
    cancelRequested: state.cancelRequested,
    found: state.found,
    uploaded: state.uploaded,
    skipped: state.skipped,
    failed: state.failed,
    errors: state.errors.slice(0, 20),
    failedFiles: (state.failedFiles || []).slice(0, 20),
    syncDir,
    target
  });
  syncHistory = history.slice(0, MAX_SYNC_HISTORY);
  await saveSyncHistory();
}

async function collectAudioFiles(root) {
  const out = [];
  await walk(root, out);
  return out;
}

async function walk(dir, out) {
  let entries;
  try {
    entries = await fs.readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }

  for (const entry of entries) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(file, out);
    } else if (entry.isFile() && /\.(mp3|wav|flac|m4a|ogg|opus|aac)$/i.test(entry.name)) {
      out.push(file);
    }
  }
}
