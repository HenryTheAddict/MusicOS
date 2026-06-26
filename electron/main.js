const { app, BrowserWindow, dialog, ipcMain } = require("electron");
const fs = require("node:fs/promises");
const path = require("node:path");

const DEFAULT_TARGET = process.env.MUSICOS_SYNC_TARGET || "http://127.0.0.1:18080";
const DEFAULT_DIR = process.env.MUSICOS_SYNC_DIR || path.resolve(process.cwd(), "music");

let mainWindow;
let syncDir = DEFAULT_DIR;
let target = DEFAULT_TARGET;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 860,
    minWidth: 1040,
    minHeight: 720,
    title: "MusicOS Sync",
    backgroundColor: "#d9eeff",
    titleBarStyle: "hiddenInset",
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
  if (process.platform !== "darwin") app.quit();
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

ipcMain.handle("config:get", () => ({
  target,
  syncDir
}));

ipcMain.handle("config:set-target", (_event, nextTarget) => {
  if (typeof nextTarget === "string" && nextTarget.trim()) {
    target = nextTarget.trim().replace(/\/$/, "");
  }
  return { target, syncDir };
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
  return { target, syncDir };
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
