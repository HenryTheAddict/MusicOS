const { app, BrowserWindow, dialog, ipcMain, shell } = require("electron");
const { spawn } = require("node:child_process");
const fs = require("node:fs/promises");
const fsSync = require("node:fs");
const path = require("node:path");

const projectRoot = path.resolve(__dirname, "..");
const DEFAULT_PORT = process.env.MUSICOS_COMPANION_PORT || "18080";
const DEFAULT_TARGET = process.env.MUSICOS_SYNC_TARGET || `http://127.0.0.1:${DEFAULT_PORT}`;
const DEFAULT_DIR = process.env.MUSICOS_SYNC_DIR || path.resolve(projectRoot, "music");
const DEFAULT_DB = process.env.MUSICOS_MEDIA_DB || path.resolve(projectRoot, "out", "desktop-media.sqlite3");
const MAX_LOG_LINES = 700;

let mainWindow;
let syncDir = DEFAULT_DIR;
let target = DEFAULT_TARGET;
let dbPath = DEFAULT_DB;
let emulatorChild = null;
let lastExit = null;
let logs = [];
let starting = false;

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

ipcMain.handle("folder:upload", async () => {
  const files = await collectAudioFiles(syncDir);
  let uploaded = 0;
  const errors = [];

  for (const file of files) {
    try {
      const body = await fs.readFile(file);
      const response = await fetch(`${target}/api/upload`, {
        method: "POST",
        headers: {
          "X-Filename": path.basename(file)
        },
        body
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      uploaded += 1;
    } catch (error) {
      errors.push(`${path.basename(file)}: ${error.message}`);
    }
  }

  return {
    folder: syncDir,
    found: files.length,
    uploaded,
    errors
  };
});

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
        MUSICOS_SYNC_DIR: syncDir,
        MUSICOS_SYNC_TARGET: target
      },
      windowsHide: true
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
