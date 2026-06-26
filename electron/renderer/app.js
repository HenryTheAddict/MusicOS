const state = {
  target: "http://127.0.0.1:18080",
  syncDir: "./music",
  dbPath: "out/desktop-media.sqlite3",
  activeTab: "overview",
  tracks: [],
  playlists: [],
  playlistTracks: [],
  selectedPlaylistId: null,
  settings: null,
  library: null,
  player: null,
  queue: [],
  diagnostics: null,
  screenshotUrl: null,
  connected: false,
  emulator: null,
  logs: [],
  busy: false,
  query: ""
};

const els = {
  statusText: document.getElementById("statusText"),
  connectionPill: document.getElementById("connectionPill"),
  emulatorPill: document.getElementById("emulatorPill"),
  trackCount: document.getElementById("trackCount"),
  albumCount: document.getElementById("albumCount"),
  playlistCount: document.getElementById("playlistCount"),
  coverArtCount: document.getElementById("coverArtCount"),
  tracks: document.getElementById("tracks"),
  playlistList: document.getElementById("playlistList"),
  playlistTracks: document.getElementById("playlistTracks"),
  selectedPlaylistTitle: document.getElementById("selectedPlaylistTitle"),
  mirror: document.getElementById("mirror"),
  mirrorEmpty: document.getElementById("mirrorEmpty"),
  actionText: document.getElementById("actionText"),
  targetInput: document.getElementById("targetInput"),
  folderText: document.getElementById("folderText"),
  dbText: document.getElementById("dbText"),
  refreshBtn: document.getElementById("refreshBtn"),
  startEmuBtn: document.getElementById("startEmuBtn"),
  restartEmuBtn: document.getElementById("restartEmuBtn"),
  stopEmuBtn: document.getElementById("stopEmuBtn"),
  chooseBtn: document.getElementById("chooseBtn"),
  openFolderBtn: document.getElementById("openFolderBtn"),
  resetDbBtn: document.getElementById("resetDbBtn"),
  uploadBtn: document.getElementById("uploadBtn"),
  userNameInput: document.getElementById("userNameInput"),
  deviceNameInput: document.getElementById("deviceNameInput"),
  audioBackendInput: document.getElementById("audioBackendInput"),
  alsaDeviceInput: document.getElementById("alsaDeviceInput"),
  saveSettingsBtn: document.getElementById("saveSettingsBtn"),
  scanBtn: document.getElementById("scanBtn"),
  audioTestBtn: document.getElementById("audioTestBtn"),
  diagnosticsBtn: document.getElementById("diagnosticsBtn"),
  clearLogsBtn: document.getElementById("clearLogsBtn"),
  playerText: document.getElementById("playerText"),
  nowPlaying: document.getElementById("nowPlaying"),
  queueList: document.getElementById("queueList"),
  diagnosticsGrid: document.getElementById("diagnosticsGrid"),
  logs: document.getElementById("logs"),
  frameText: document.getElementById("frameText"),
  librarySearch: document.getElementById("librarySearch"),
  playlistForm: document.getElementById("playlistForm"),
  playlistNameInput: document.getElementById("playlistNameInput"),
  playPlaylistBtn: document.getElementById("playPlaylistBtn"),
  renamePlaylistBtn: document.getElementById("renamePlaylistBtn"),
  deletePlaylistBtn: document.getElementById("deletePlaylistBtn")
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
  applyConfig(config);
  bindEvents();
  renderConfig();
  await refreshAll();
  setInterval(refreshLightweight, 1200);
  setInterval(refreshTracks, 3500);
  setInterval(refreshLogs, 1600);
}

function bindEvents() {
  document.querySelectorAll(".tab").forEach(button => {
    button.addEventListener("click", () => setTab(button.dataset.tab));
  });

  els.refreshBtn.addEventListener("click", refreshAll);
  els.startEmuBtn.addEventListener("click", startEmulator);
  els.restartEmuBtn.addEventListener("click", restartEmulator);
  els.stopEmuBtn.addEventListener("click", stopEmulator);
  els.chooseBtn.addEventListener("click", chooseFolder);
  els.openFolderBtn.addEventListener("click", openFolder);
  els.resetDbBtn.addEventListener("click", resetDb);
  els.uploadBtn.addEventListener("click", uploadFolder);
  els.saveSettingsBtn.addEventListener("click", saveSettings);
  els.scanBtn.addEventListener("click", scanLibrary);
  els.audioTestBtn.addEventListener("click", audioTest);
  els.diagnosticsBtn.addEventListener("click", showDiagnostics);
  els.clearLogsBtn.addEventListener("click", clearLogs);
  els.playPlaylistBtn.addEventListener("click", playSelectedPlaylist);
  els.renamePlaylistBtn.addEventListener("click", renameSelectedPlaylist);
  els.deletePlaylistBtn.addEventListener("click", deleteSelectedPlaylist);
  els.playlistForm.addEventListener("submit", createPlaylist);
  els.librarySearch.addEventListener("input", event => {
    state.query = event.target.value.trim().toLowerCase();
    renderTracks();
  });

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
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
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

function setTab(tab) {
  state.activeTab = tab;
  document.querySelectorAll(".tab").forEach(button => {
    button.classList.toggle("active", button.dataset.tab === tab);
  });
  document.querySelectorAll(".view").forEach(view => {
    view.classList.toggle("active", view.dataset.view === tab);
  });
}

function applyConfig(config) {
  state.target = config.target;
  state.syncDir = config.syncDir;
  state.dbPath = config.dbPath || state.dbPath;
  state.emulator = config.emulator || state.emulator;
}

function renderConfig() {
  els.targetInput.value = state.target;
  els.folderText.textContent = state.syncDir;
  els.dbText.textContent = state.dbPath;
  renderEmulator();
}

async function refreshAll() {
  setAction("Refreshing MusicOS...");
  await Promise.allSettled([
    refreshEmulatorStatus(),
    refreshLibrary(),
    refreshSettings(),
    refreshTracks(),
    refreshPlaylists(),
    refreshMirror(),
    refreshLogs()
  ]);
  renderAll();
}

async function refreshLightweight() {
  await Promise.allSettled([refreshEmulatorStatus(), refreshLibrary(), refreshMirror()]);
  renderAll();
}

async function refreshEmulatorStatus() {
  try {
    state.emulator = await window.musicos.emulator.status();
  } catch (error) {
    setAction(`Emulator status failed: ${error.message}`);
  }
}

async function refreshLibrary() {
  try {
    const data = await getJson("/api/library");
    state.library = data;
    state.player = data.player || null;
    state.queue = Array.isArray(data.queue) ? data.queue : [];
    state.diagnostics = data;
    if (Array.isArray(data.playlists)) {
      state.playlists = data.playlists;
      preserveSelectedPlaylist();
    }
    setConnected(true, data.status === "ok" ? "Connected" : "Warming up");
  } catch {
    state.library = null;
    state.player = null;
    state.queue = [];
    setConnected(false, "Waiting for MusicOS");
  }
}

async function refreshSettings() {
  try {
    const settings = await getJson("/api/settings");
    state.settings = settings;
    if (document.activeElement !== els.userNameInput) els.userNameInput.value = settings.user_name || "";
    if (document.activeElement !== els.deviceNameInput) els.deviceNameInput.value = settings.device_name || "";
    if (document.activeElement !== els.audioBackendInput) els.audioBackendInput.value = settings.audio_backend || "auto";
    if (document.activeElement !== els.alsaDeviceInput) els.alsaDeviceInput.value = settings.alsa_device || "";
  } catch {
    state.settings = null;
  }
}

async function refreshTracks() {
  try {
    const tracks = await getJson("/api/tracks");
    state.tracks = Array.isArray(tracks) ? tracks : [];
    setConnected(true, "Library live");
    renderTracks();
  } catch {
    renderTracks();
  }
}

async function refreshPlaylists() {
  try {
    const playlists = await getJson("/api/playlists");
    state.playlists = Array.isArray(playlists) ? playlists : [];
    preserveSelectedPlaylist();
    await refreshSelectedPlaylistTracks();
  } catch {
    state.playlists = [];
    state.playlistTracks = [];
  }
}

async function refreshSelectedPlaylistTracks() {
  if (!state.selectedPlaylistId) {
    state.playlistTracks = [];
    return;
  }
  try {
    const tracks = await getJson(`/api/playlists/${state.selectedPlaylistId}/tracks`);
    state.playlistTracks = Array.isArray(tracks) ? tracks : [];
  } catch {
    state.playlistTracks = [];
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

async function refreshLogs() {
  try {
    const result = await window.musicos.emulator.logs();
    state.logs = result.logs || [];
    state.emulator = result.status || state.emulator;
    renderLogs();
    renderEmulator();
  } catch {}
}

function renderAll() {
  renderConfig();
  renderCounts();
  renderPlayer();
  renderQueue();
  renderTracks();
  renderPlaylists();
  renderDiagnostics();
  renderLogs();
}

function renderCounts() {
  const counts = state.library?.counts || {};
  els.trackCount.textContent = counts.tracks ?? state.tracks.length;
  els.albumCount.textContent = counts.albums ?? "--";
  els.playlistCount.textContent = counts.playlists ?? state.playlists.length;
  els.coverArtCount.textContent = state.library?.cover_art_cache_count ?? "--";
  els.frameText.textContent = `${Number(state.library?.frame_ms || 0).toFixed(1)} ms`;
}

function renderPlayer() {
  const track = state.player?.track;
  if (track) {
    const prefix = state.player.is_playing ? "Playing" : "Paused";
    els.playerText.textContent = `${prefix} / ${state.queue.length} queued`;
    els.nowPlaying.replaceChildren(trackArt(track, "now-art"), trackBody(track, prefix));
  } else {
    els.playerText.textContent = state.queue.length ? `${state.queue.length} queued` : "Queue idle";
    els.nowPlaying.innerHTML = `<div class="empty">No track selected</div>`;
  }
}

function renderQueue() {
  if (!state.queue.length) {
    els.queueList.innerHTML = `<div class="empty">Queue empty</div>`;
    return;
  }
  els.queueList.replaceChildren(
    ...state.queue.slice(0, 8).map((track, index) => {
      const row = document.createElement("div");
      row.className = "queue-row";
      row.textContent = `${String(index + 1).padStart(2, "0")}  ${track.title} / ${track.artist}`;
      return row;
    })
  );
}

function renderTracks() {
  const tracks = filteredTracks();
  if (!tracks.length) {
    els.tracks.innerHTML = `<div class="empty">${state.query ? "No matching tracks" : "No tracks indexed"}</div>`;
    return;
  }
  els.tracks.replaceChildren(...tracks.slice(0, 80).map((track, index) => trackRow(track, index)));
}

function renderPlaylists() {
  if (!state.playlists.length) {
    els.playlistList.innerHTML = `<div class="empty">No playlists</div>`;
  } else {
    els.playlistList.replaceChildren(
      ...state.playlists.map(playlist => {
        const button = document.createElement("button");
        button.className = "playlist-row";
        button.classList.toggle("active", playlist.id === state.selectedPlaylistId);
        button.type = "button";
        const count = playlist.id === state.selectedPlaylistId ? state.playlistTracks.length : "";
        button.innerHTML = `<span class="playlist-title"></span><span class="playlist-meta">${count ? `${count} tracks` : "Playlist"}</span>`;
        button.querySelector(".playlist-title").textContent = playlist.name;
        button.addEventListener("click", async () => {
          state.selectedPlaylistId = playlist.id;
          await refreshSelectedPlaylistTracks();
          renderPlaylists();
        });
        return button;
      })
    );
  }

  const selected = selectedPlaylist();
  els.selectedPlaylistTitle.textContent = selected?.name || "No Playlist";
  els.playPlaylistBtn.disabled = !selected;
  els.renamePlaylistBtn.disabled = !selected;
  els.deletePlaylistBtn.disabled = !selected;

  if (!selected) {
    els.playlistTracks.innerHTML = `<div class="empty">Select a playlist</div>`;
  } else if (!state.playlistTracks.length) {
    els.playlistTracks.innerHTML = `<div class="empty">Playlist empty</div>`;
  } else {
    els.playlistTracks.replaceChildren(
      ...state.playlistTracks.map((track, index) => trackRow(track, index, { playlist: selected.id }))
    );
  }
}

function renderDiagnostics() {
  const audio = state.diagnostics?.audio || {};
  const items = [
    ["Audio", state.player?.backend || audio.backend_env || "unknown"],
    ["PCM", state.player?.selected_pcm || audio.pcm_candidates?.[0] || "none"],
    ["Scan", state.library?.scan_status || "unknown"],
    ["Roots", (state.library?.indexed_roots || []).length],
    ["Battery", state.library?.battery_status || "unavailable"],
    ["Errors", state.player?.last_error || "none"]
  ];
  els.diagnosticsGrid.replaceChildren(
    ...items.map(([label, value]) => {
      const div = document.createElement("div");
      div.className = "diagnostic";
      div.innerHTML = `<span class="eyebrow"></span><strong></strong>`;
      div.querySelector(".eyebrow").textContent = label;
      div.querySelector("strong").textContent = String(value);
      return div;
    })
  );
}

function renderLogs() {
  if (!state.logs.length) {
    els.logs.innerHTML = `<div class="log-row">No logs yet</div>`;
    return;
  }
  els.logs.replaceChildren(
    ...state.logs.slice(-220).map(entry => {
      const row = document.createElement("div");
      row.className = "log-row";
      const time = new Date(entry.at).toLocaleTimeString();
      row.innerHTML = `<span class="log-source"></span> <span></span>`;
      row.querySelector(".log-source").textContent = `[${time} ${entry.source}]`;
      row.querySelector("span:last-child").textContent = ` ${entry.line}`;
      return row;
    })
  );
  els.logs.scrollTop = els.logs.scrollHeight;
}

function renderEmulator() {
  const running = Boolean(state.emulator?.running);
  const starting = Boolean(state.emulator?.starting);
  els.emulatorPill.textContent = starting ? "Starting" : running ? `Emulator ${state.emulator.pid}` : "Emulator off";
  els.emulatorPill.classList.toggle("online", running);
  els.emulatorPill.classList.toggle("warn", starting);
  els.emulatorPill.classList.toggle("offline", !running && !starting);
  els.startEmuBtn.disabled = running || starting;
  els.restartEmuBtn.disabled = starting;
  els.stopEmuBtn.disabled = !running;
}

function trackRow(track, index, options = {}) {
  const row = document.createElement("div");
  row.className = "track";
  const number = document.createElement("span");
  number.className = "track-index";
  number.textContent = String(index + 1).padStart(2, "0");

  const body = trackBody(track);
  const actions = document.createElement("span");
  actions.className = "track-actions";
  actions.append(
    actionButton("Play", () => playTrack(track.id)),
    actionButton("Next", () => queueTrack(track.id, "next")),
    actionButton(track.favorited ? "Unfav" : "Fav", () => toggleFavorite(track.id))
  );
  if (state.selectedPlaylistId && !options.playlist) {
    actions.append(actionButton("Add", () => addTrackToSelectedPlaylist(track.id)));
  }
  if (options.playlist) {
    actions.append(actionButton("Remove", () => removeTrackFromPlaylist(options.playlist, track.id)));
  }

  row.append(number, trackArt(track, "track-art"), body, actions);
  return row;
}

function trackArt(track, className) {
  const img = document.createElement("img");
  img.className = className;
  img.alt = "";
  img.loading = "lazy";
  img.src = `${state.target}/api/cover-art.bmp?id=${encodeURIComponent(track.id)}&v=${track.cover_art_updated_at || 0}`;
  img.addEventListener("error", () => {
    img.removeAttribute("src");
    img.classList.add("art-missing");
  });
  return img;
}

function trackBody(track, prefix = "") {
  const body = document.createElement("span");
  const title = document.createElement("span");
  title.className = prefix ? "now-title" : "track-title";
  title.textContent = prefix ? `${prefix}: ${track.title || fileTitle(track.path) || "Untitled"}` : track.title || fileTitle(track.path) || "Untitled";
  const meta = document.createElement("span");
  meta.className = prefix ? "now-meta" : "track-meta";
  meta.textContent = [track.artist, track.album, formatDuration(track.duration)].filter(Boolean).join(" / ");
  body.append(title, meta);
  return body;
}

function actionButton(label, onClick) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.addEventListener("click", onClick);
  return button;
}

function filteredTracks() {
  if (!state.query) return state.tracks;
  return state.tracks.filter(track =>
    [track.title, track.artist, track.album, track.path]
      .filter(Boolean)
      .some(value => value.toLowerCase().includes(state.query))
  );
}

function preserveSelectedPlaylist() {
  if (state.selectedPlaylistId && state.playlists.some(playlist => playlist.id === state.selectedPlaylistId)) {
    return;
  }
  state.selectedPlaylistId = state.playlists[0]?.id || null;
}

function selectedPlaylist() {
  return state.playlists.find(playlist => playlist.id === state.selectedPlaylistId) || null;
}

async function startEmulator() {
  await guarded("Starting emulator...", async () => {
    state.emulator = await window.musicos.emulator.start();
    await refreshAll();
  });
}

async function stopEmulator() {
  await guarded("Stopping emulator...", async () => {
    state.emulator = await window.musicos.emulator.stop();
    renderEmulator();
  });
}

async function restartEmulator() {
  await guarded("Restarting emulator...", async () => {
    state.emulator = await window.musicos.emulator.restart();
    await refreshAll();
  });
}

async function chooseFolder() {
  const config = await window.musicos.chooseFolder();
  applyConfig(config);
  renderConfig();
  setAction("Music folder selected.");
}

async function openFolder() {
  await window.musicos.openFolder();
  setAction("Folder opened.");
}

async function resetDb() {
  if (!window.confirm("Reset the desktop media database?")) return;
  await guarded("Resetting media database...", async () => {
    const result = await window.musicos.resetDb();
    applyConfig(result);
    state.tracks = [];
    state.playlists = [];
    state.playlistTracks = [];
    await refreshAll();
  });
}

async function uploadFolder() {
  if (state.busy) return;
  state.busy = true;
  els.uploadBtn.disabled = true;
  try {
    setAction("Uploading local music...");
    const result = await window.musicos.uploadFolder();
    const suffix = result.errors?.length ? ` / ${result.errors.length} errors` : "";
    setAction(`Uploaded ${result.uploaded}/${result.found}${suffix}.`);
    await Promise.allSettled([refreshLibrary(), refreshTracks()]);
    renderAll();
  } catch (error) {
    setAction(`Upload failed: ${error.message}`);
  } finally {
    state.busy = false;
    els.uploadBtn.disabled = false;
  }
}

async function saveTarget() {
  const nextTarget = els.targetInput.value.trim();
  if (!nextTarget || nextTarget === state.target) return;
  const config = await window.musicos.setTarget(nextTarget);
  applyConfig(config);
  renderConfig();
  await refreshAll();
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
  await Promise.allSettled([refreshSettings(), refreshLibrary()]);
  renderAll();
}

async function scanLibrary() {
  await postJson("/api/control", { command: "scan_library" });
  setAction("Library scan requested.");
  await Promise.allSettled([refreshLibrary(), refreshTracks()]);
  renderAll();
}

async function audioTest() {
  await postJson("/api/control", { command: "audio_test" });
  setAction("Audio test requested.");
  await Promise.allSettled([refreshLibrary(), refreshLogs()]);
  renderAll();
}

async function showDiagnostics() {
  await refreshLibrary();
  setTab("device");
  setAction("Diagnostics refreshed.");
}

async function clearLogs() {
  const result = await window.musicos.emulator.clearLogs();
  state.logs = result.logs || [];
  renderLogs();
  setAction("Logs cleared.");
}

async function createPlaylist(event) {
  event.preventDefault();
  const name = els.playlistNameInput.value.trim();
  if (!name) return;
  await postJson("/api/playlists", { name });
  els.playlistNameInput.value = "";
  await refreshPlaylists();
  renderPlaylists();
  setAction("Playlist created.");
}

async function renameSelectedPlaylist() {
  const selected = selectedPlaylist();
  if (!selected) return;
  const name = window.prompt("Playlist name", selected.name);
  if (!name?.trim()) return;
  await putJson(`/api/playlists/${selected.id}`, { name: name.trim() });
  await refreshPlaylists();
  renderPlaylists();
  setAction("Playlist renamed.");
}

async function deleteSelectedPlaylist() {
  const selected = selectedPlaylist();
  if (!selected || !window.confirm(`Delete ${selected.name}?`)) return;
  await deleteJson(`/api/playlists/${selected.id}`);
  state.selectedPlaylistId = null;
  await refreshPlaylists();
  renderPlaylists();
  setAction("Playlist deleted.");
}

async function playSelectedPlaylist() {
  const selected = selectedPlaylist();
  if (!selected) return;
  await postJson("/api/control", { command: "play_playlist", id: selected.id, start_index: 0 });
  await refreshLibrary();
  renderAll();
  setAction("Playlist started.");
}

async function addTrackToSelectedPlaylist(id) {
  if (!state.selectedPlaylistId) return;
  await postJson(`/api/playlists/${state.selectedPlaylistId}/tracks`, { track_id: id });
  await refreshSelectedPlaylistTracks();
  renderPlaylists();
  setAction("Track added.");
}

async function removeTrackFromPlaylist(playlistId, trackId) {
  await deleteJson(`/api/playlists/${playlistId}/tracks/${trackId}`);
  await refreshSelectedPlaylistTracks();
  renderPlaylists();
  setAction("Track removed.");
}

async function toggleFavorite(id) {
  await postJson("/api/control", { command: "toggle_favorite", id });
  await Promise.allSettled([refreshTracks(), refreshLibrary()]);
  renderAll();
  setAction("Favorite updated.");
}

async function sendControl(command) {
  await postJson("/api/control", { command });
  setAction(`${labelForCommand(command)} sent.`);
  await refreshLibrary();
  renderAll();
}

async function playTrack(id) {
  await postJson("/api/control", { command: "play_track", id });
  await refreshLibrary();
  renderAll();
  setAction("Track started.");
}

async function queueTrack(id, position) {
  await postJson("/api/control", { command: "queue_add", id, position });
  await refreshLibrary();
  renderAll();
  setAction(position === "next" ? "Added next." : "Added later.");
}

async function sendInput(key, rawKey) {
  flashKey(rawKey);
  await fetch(`${state.target}/api/input?key=${encodeURIComponent(key)}`, { method: "POST" });
  setAction(`${key} sent.`);
}

async function guarded(message, fn) {
  try {
    setAction(message);
    await fn();
  } catch (error) {
    setAction(error.message);
  }
}

async function getJson(path) {
  const response = await fetch(`${state.target}${path}`);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

async function postJson(path, body) {
  return sendJson("POST", path, body);
}

async function putJson(path, body) {
  return sendJson("PUT", path, body);
}

async function deleteJson(path) {
  const response = await fetch(`${state.target}${path}`, { method: "DELETE" });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
}

async function sendJson(method, path, body) {
  const response = await fetch(`${state.target}${path}`, {
    method,
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

boot().catch(error => setAction(`Control Center failed: ${error.message}`));
