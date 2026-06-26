const state = {
  target: "http://127.0.0.1:18080",
  syncDir: "./music",
  tracks: [],
  settings: null,
  player: null,
  diagnostics: null,
  screenshotUrl: null,
  connected: false,
  busy: false
};

const els = {
  statusText: document.getElementById("statusText"),
  connectionPill: document.getElementById("connectionPill"),
  trackCount: document.getElementById("trackCount"),
  albumCount: document.getElementById("albumCount"),
  playlistCount: document.getElementById("playlistCount"),
  pinCount: document.getElementById("pinCount"),
  tracks: document.getElementById("tracks"),
  mirror: document.getElementById("mirror"),
  mirrorEmpty: document.getElementById("mirrorEmpty"),
  actionText: document.getElementById("actionText"),
  targetInput: document.getElementById("targetInput"),
  folderText: document.getElementById("folderText"),
  refreshBtn: document.getElementById("refreshBtn"),
  chooseBtn: document.getElementById("chooseBtn"),
  uploadBtn: document.getElementById("uploadBtn"),
  usbBtn: document.getElementById("usbBtn"),
  userNameInput: document.getElementById("userNameInput"),
  deviceNameInput: document.getElementById("deviceNameInput"),
  audioBackendInput: document.getElementById("audioBackendInput"),
  alsaDeviceInput: document.getElementById("alsaDeviceInput"),
  saveSettingsBtn: document.getElementById("saveSettingsBtn"),
  scanBtn: document.getElementById("scanBtn"),
  audioTestBtn: document.getElementById("audioTestBtn"),
  queueClearBtn: document.getElementById("queueClearBtn"),
  diagnosticsBtn: document.getElementById("diagnosticsBtn"),
  playerText: document.getElementById("playerText")
};

const keyMap = new Map([
  ["w", "Up"],
  ["arrowup", "Up"],
  ["a", "Left"],
  ["arrowleft", "Left"],
  ["s", "Down"],
  ["arrowdown", "Down"],
  ["d", "Right"],
  ["arrowright", "Right"],
  ["j", "Confirm"],
  ["enter", "Confirm"],
  [" ", "Confirm"],
  ["h", "Back"],
  ["backspace", "Back"],
  ["u", "Context"],
  ["i", "Home"]
]);

async function boot() {
  const config = await window.musicos.getConfig();
  state.target = config.target;
  state.syncDir = config.syncDir;
  renderConfig();
  bindEvents();
  await refreshAll();
  setInterval(refreshStatus, 1500);
  setInterval(refreshPlayer, 1500);
  setInterval(refreshTracks, 2600);
  setInterval(refreshMirror, 760);
}

function bindEvents() {
  els.refreshBtn.addEventListener("click", refreshAll);
  els.chooseBtn.addEventListener("click", chooseFolder);
  els.uploadBtn.addEventListener("click", uploadFolder);
  els.usbBtn.addEventListener("click", usbSync);
  els.saveSettingsBtn.addEventListener("click", saveSettings);
  els.scanBtn.addEventListener("click", scanLibrary);
  els.audioTestBtn.addEventListener("click", audioTest);
  els.queueClearBtn.addEventListener("click", clearQueue);
  els.diagnosticsBtn.addEventListener("click", showDiagnostics);
  els.targetInput.addEventListener("keydown", async event => {
    if (event.key === "Enter") {
      event.preventDefault();
      await saveTarget();
      els.targetInput.blur();
    }
  });
  els.targetInput.addEventListener("blur", saveTarget);

  document.querySelectorAll("[data-control]").forEach(button => {
    button.addEventListener("click", () => sendControl(button.dataset.control));
  });

  window.addEventListener("keydown", event => {
    if (event.target instanceof HTMLInputElement) return;
    const key = event.key.toLowerCase();
    if (key === "r") {
      event.preventDefault();
      refreshAll();
      return;
    }
    const mapped = keyMap.get(key);
    if (mapped) {
      event.preventDefault();
      sendInput(mapped, event.key);
    }
  });
}

async function saveTarget() {
  const nextTarget = els.targetInput.value.trim();
  if (!nextTarget || nextTarget === state.target) return;
  const config = await window.musicos.setTarget(nextTarget);
  state.target = config.target;
  renderConfig();
  await refreshAll();
}

function renderConfig() {
  els.targetInput.value = state.target;
  els.folderText.textContent = state.syncDir;
}

async function refreshAll() {
  setAction("Refreshing MusicOS...");
  await Promise.allSettled([
    refreshStatus(),
    refreshSettings(),
    refreshPlayer(),
    refreshTracks(),
    refreshMirror()
  ]);
}

async function refreshStatus() {
  try {
    const data = await getJson("/api/diagnostics");
    state.diagnostics = data;
    const counts = data.counts || {};
    els.trackCount.textContent = counts.tracks ?? state.tracks.length;
    els.albumCount.textContent = counts.albums ?? "--";
    els.playlistCount.textContent = counts.playlists ?? "--";
    els.pinCount.textContent = counts.favorites ?? "--";
    setConnected(true, "Connected");
  } catch {
    setConnected(false, "Waiting for MusicOS");
  }
}

async function refreshSettings() {
  try {
    const settings = await getJson("/api/settings");
    state.settings = settings;
    if (document.activeElement !== els.userNameInput) {
      els.userNameInput.value = settings.user_name || "";
    }
    if (document.activeElement !== els.deviceNameInput) {
      els.deviceNameInput.value = settings.device_name || "";
    }
    if (document.activeElement !== els.audioBackendInput) {
      els.audioBackendInput.value = settings.audio_backend || "auto";
    }
    if (document.activeElement !== els.alsaDeviceInput) {
      els.alsaDeviceInput.value = settings.alsa_device || "";
    }
  } catch {
    state.settings = null;
  }
}

async function refreshPlayer() {
  try {
    const data = await getJson("/api/player");
    state.player = data.player;
    const track = data.player?.track;
    const queueLength = data.queue?.length ?? data.player?.queue_len ?? 0;
    if (track) {
      const prefix = data.player.is_playing ? "Playing" : "Paused";
      els.playerText.textContent = `${prefix}: ${track.title} (${queueLength} queued)`;
    } else if (queueLength > 0) {
      els.playerText.textContent = `${queueLength} queued`;
    } else {
      els.playerText.textContent = "Queue idle";
    }
  } catch {
    state.player = null;
    els.playerText.textContent = "Queue idle";
  }
}

async function refreshTracks() {
  try {
    const tracks = await getJson("/api/tracks");
    state.tracks = Array.isArray(tracks) ? tracks : [];
    renderTracks();
    setConnected(true, "Library live");
  } catch {
    if (!state.connected) renderTracks();
  }
}

async function refreshMirror() {
  try {
    const response = await fetch(`${state.target}/api/screenshot.bmp?t=${Date.now()}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const blob = await response.blob();
    const nextUrl = URL.createObjectURL(blob);
    const oldUrl = state.screenshotUrl;
    state.screenshotUrl = nextUrl;
    els.mirror.src = nextUrl;
    els.mirror.style.display = "block";
    els.mirrorEmpty.style.display = "none";
    if (oldUrl) URL.revokeObjectURL(oldUrl);
    setConnected(true, "Mirror live");
  } catch {
    if (!state.screenshotUrl) {
      els.mirror.style.display = "none";
      els.mirrorEmpty.style.display = "grid";
    }
  }
}

function renderTracks() {
  if (!state.tracks.length) {
    els.tracks.innerHTML = `<div class="empty">Drop music into the sync folder, then upload.</div>`;
    return;
  }

  els.tracks.replaceChildren(
    ...state.tracks.slice(0, 24).map((track, index) => {
      const row = document.createElement("div");
      row.className = "track";

      const number = document.createElement("span");
      number.className = "track-index";
      number.textContent = String(index + 1).padStart(2, "0");

      const art = document.createElement("img");
      art.className = "track-art";
      art.alt = "";
      art.loading = "lazy";
      art.src = `${state.target}/api/cover-art.bmp?id=${encodeURIComponent(track.id)}`;
      art.addEventListener("error", () => {
        art.removeAttribute("src");
        art.classList.add("track-art-missing");
      });

      const body = document.createElement("span");
      const title = document.createElement("span");
      title.className = "track-title";
      title.textContent = track.title || fileTitle(track.path) || "Untitled";
      const meta = document.createElement("span");
      meta.className = "track-meta";
      meta.textContent = [track.artist, track.album, formatDuration(track.duration)]
        .filter(Boolean)
        .join(" / ");
      body.append(title, meta);

      const actions = document.createElement("span");
      actions.className = "track-actions";
      actions.append(
        trackAction("Play", () => playTrack(track.id)),
        trackAction("Next", () => queueTrack(track.id, "next")),
        trackAction("Later", () => queueTrack(track.id, "later"))
      );

      row.append(number, art, body, actions);
      return row;
    })
  );
}

function trackAction(label, onClick) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.addEventListener("click", onClick);
  return button;
}

async function chooseFolder() {
  const config = await window.musicos.chooseFolder();
  state.target = config.target;
  state.syncDir = config.syncDir;
  renderConfig();
  setAction("Sync folder selected.");
}

async function uploadFolder() {
  if (state.busy) return;
  setBusy(true);
  try {
    setAction("Uploading local music...");
    const result = await window.musicos.uploadFolder();
    const suffix = result.errors?.length ? ` (${result.errors.length} errors)` : "";
    setAction(`Uploaded ${result.uploaded}/${result.found} tracks${suffix}.`);
    await Promise.allSettled([refreshStatus(), refreshTracks()]);
  } catch (error) {
    setAction(`Upload failed: ${error.message}`);
  } finally {
    setBusy(false);
  }
}

async function usbSync() {
  await postJson("/api/sync-usb", {});
  setAction("USB sync requested.");
  await Promise.allSettled([refreshStatus(), refreshTracks()]);
}

async function saveSettings() {
  const body = {
    user_name: els.userNameInput.value.trim(),
    device_name: els.deviceNameInput.value.trim(),
    audio_backend: els.audioBackendInput.value,
    alsa_device: els.alsaDeviceInput.value.trim()
  };
  await postJson("/api/settings", body);
  setAction("Settings saved.");
  await Promise.allSettled([refreshSettings(), refreshStatus()]);
}

async function scanLibrary() {
  await postJson("/api/control", { command: "scan_library" });
  setAction("Library scan requested.");
  await Promise.allSettled([refreshStatus(), refreshTracks()]);
}

async function audioTest() {
  await postJson("/api/control", { command: "audio_test" });
  setAction("Audio test requested.");
  await Promise.allSettled([refreshStatus(), refreshPlayer()]);
}

async function clearQueue() {
  await postJson("/api/control", { command: "queue_clear" });
  setAction("Queue cleared.");
  await refreshPlayer();
}

async function showDiagnostics() {
  try {
    const diagnostics = await getJson("/api/diagnostics");
    state.diagnostics = diagnostics;
    const audio = diagnostics.audio || {};
    const firstPcm = audio.pcm_candidates?.[0] || "no PCM";
    setAction(`Diagnostics: ${diagnostics.db_path || "DB"} / ${firstPcm}`);
  } catch (error) {
    setAction(`Diagnostics failed: ${error.message}`);
  }
}

async function sendControl(command) {
  await postJson("/api/control", { command });
  setAction(`${labelForCommand(command)} sent.`);
  await refreshPlayer();
}

async function playTrack(id) {
  await postJson("/api/control", { command: "play_track", id });
  setAction("Track queued on MusicOS.");
  await refreshPlayer();
}

async function queueTrack(id, position) {
  await postJson("/api/control", { command: "queue_add", id, position });
  setAction(position === "next" ? "Added next." : "Added later.");
  await refreshPlayer();
}

async function sendInput(key, rawKey) {
  flashKey(rawKey);
  await fetch(`${state.target}/api/input?key=${encodeURIComponent(key)}`, { method: "POST" });
  setAction(`${key} sent.`);
}

async function getJson(path) {
  const response = await fetch(`${state.target}${path}`);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

async function postJson(path, body) {
  const response = await fetch(`${state.target}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
}

function setConnected(connected, text) {
  state.connected = connected;
  els.statusText.textContent = text;
  els.connectionPill.textContent = connected ? "Online" : "Offline";
  els.connectionPill.classList.toggle("online", connected);
  els.connectionPill.classList.toggle("offline", !connected);
}

function setBusy(busy) {
  state.busy = busy;
  els.uploadBtn.disabled = busy;
  els.uploadBtn.textContent = busy ? "Uploading..." : "Upload Music";
}

function setAction(text) {
  els.actionText.textContent = text;
}

function flashKey(key) {
  const label = key === " " ? "J" : key.toUpperCase();
  document.querySelectorAll("kbd").forEach(kbd => {
    if (kbd.textContent === label || (label === "ENTER" && kbd.textContent === "J")) {
      kbd.classList.add("key-hit");
      setTimeout(() => kbd.classList.remove("key-hit"), 180);
    }
  });
}

function fileTitle(path) {
  return path?.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "");
}

function formatDuration(seconds) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

function labelForCommand(command) {
  return command
    .split("_")
    .map(part => part[0].toUpperCase() + part.slice(1))
    .join(" ");
}

boot().catch(error => setAction(`Sync app failed: ${error.message}`));
