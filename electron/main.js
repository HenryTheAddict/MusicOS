const { app, BrowserWindow, dialog, ipcMain } = require("electron");
const fs = require("node:fs/promises");
const fsSync = require("node:fs");
const path = require("node:path");

const CONFIG_PATH = path.join(
  app.getPath("userData"),
  "sync-config.json"
);
const DEFAULT_TARGET =
  process.env.MUSICOS_SYNC_TARGET || "http://127.0.0.1:18080";
const DEFAULT_DIR =
  process.env.MUSICOS_SYNC_DIR || path.resolve(process.cwd(), "music");

let mainWindow;
let config = loadConfig();

/* ── persistent config ─────────────────────────── */

function loadConfig() {
  try {
    const raw = fsSync.readFileSync(CONFIG_PATH, "utf-8");
    const parsed = JSON.parse(raw);
    return {
      target: parsed.target || DEFAULT_TARGET,
      syncDir: parsed.syncDir || DEFAULT_DIR,
    };
  } catch {
    return { target: DEFAULT_TARGET, syncDir: DEFAULT_DIR };
  }
}

function saveConfig() {
  try {
    fsSync.mkdirSync(path.dirname(CONFIG_PATH), { recursive: true });
    fsSync.writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2));
  } catch {
    /* best-effort */
  }
}

/* ── window ────────────────────────────────────── */

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
      nodeIntegration: false,
    },
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

/* ── IPC handlers ──────────────────────────────── */

ipcMain.handle("config:get", () => ({
  target: config.target,
  syncDir: config.syncDir,
}));

ipcMain.handle("config:set-target", (_event, nextTarget) => {
  if (typeof nextTarget === "string" && nextTarget.trim()) {
    config.target = nextTarget.trim().replace(/\/$/, "");
    saveConfig();
  }
  return { target: config.target, syncDir: config.syncDir };
});

ipcMain.handle("folder:choose", async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: "Choose MusicOS sync folder",
    defaultPath: config.syncDir,
    properties: ["openDirectory", "createDirectory"],
  });
  if (!result.canceled && result.filePaths[0]) {
    config.syncDir = result.filePaths[0];
    saveConfig();
  }
  return { target: config.target, syncDir: config.syncDir };
});

let uploadPaused = false;

ipcMain.on("upload:pause", () => {
  uploadPaused = true;
});

ipcMain.on("upload:resume", () => {
  uploadPaused = false;
});

ipcMain.handle("folder:upload", async (event, customPath) => {
  uploadPaused = false;
  const uploadPath = customPath || config.syncDir;
  const files = await collectAudioFiles(uploadPath);
  let uploaded = 0;
  const errors = [];
  const total = files.length;
  const sender = event.sender;

  for (let i = 0; i < files.length; i++) {
    while (uploadPaused) {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }

    const file = files[i];
    const filename = path.basename(file);
    try {
      sender.send("upload:progress", {
        current: i + 1,
        total,
        filename,
        phase: "uploading",
      });
      const body = await fs.readFile(file);

      let attempts = 0;
      let success = false;
      while (!success && !uploadPaused) {
        try {
          const response = await fetch(`${config.target}/api/upload`, {
            method: "POST",
            headers: { "X-Filename": filename },
            body,
          });
          if (response.ok) {
            success = true;
          } else {
            throw new Error(`HTTP ${response.status}`);
          }
        } catch (err) {
          attempts += 1;
          if (attempts >= 3) {
            sender.send("upload:progress", {
              current: i,
              total,
              filename: `Waiting for connection... (${filename})`,
              phase: "paused",
            });
            uploadPaused = true;
            while (uploadPaused) {
              await new Promise((resolve) => setTimeout(resolve, 1000));
            }
            attempts = 0;
          } else {
            await new Promise((resolve) => setTimeout(resolve, 1000));
          }
        }
      }

      if (success) {
        uploaded += 1;
      } else {
        i -= 1;
      }
    } catch (error) {
      errors.push(`${filename}: ${error.message}`);
    }
  }

  sender.send("upload:progress", {
    current: total,
    total,
    filename: "",
    phase: "done",
  });

  return {
    folder: uploadPath,
    found: files.length,
    uploaded,
    errors,
  };
});

/* ── file helpers ──────────────────────────────── */

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
    } else if (
      entry.isFile() &&
      /\.(mp3|wav|flac|m4a|ogg|opus|aac)$/i.test(entry.name)
    ) {
      out.push(file);
    }
  }
}
