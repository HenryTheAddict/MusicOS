/* ── state ──────────────────────────────────────── */

const state = {
  target: "http://127.0.0.1:18080",
  syncDir: "./music",
  tracks: [],
  settings: null,
  player: null,
  diagnostics: null,
  screenshotUrl: null,
  connected: false,
  busy: false,
  searchQuery: "",
  sortBy: "title",
  _playerLock: false,
  _savingTarget: false,
  _lastPlayingId: null,
  mockMode: false,
};

/* ── element cache ─────────────────────────────── */

const els = {};

function cacheElements() {
  const ids = [
    "statusText", "connectionPill", "trackCount", "albumCount",
    "playlistCount", "pinCount", "tracks", "mirror", "mirrorEmpty",
    "targetInput", "folderText", "refreshBtn", "chooseBtn",
    "uploadBtn", "usbBtn", "userNameInput", "deviceNameInput",
    "audioBackendInput", "alsaDeviceInput", "saveSettingsBtn",
    "scanBtn", "audioTestBtn", "queueClearBtn", "diagnosticsBtn",
    "toastContainer", "searchInput", "sortSelect", "trackCounter",
    "uploadProgress", "uploadProgressText", "uploadProgressCount",
    "uploadProgressFill",
    "nowPlaying", "npArt", "npTitle", "npArtist", "npTime",
    "npProgressFill", "npPrev", "npPlayPause", "npNext",
    "mockModeCheckbox", "dropOverlay",
  ];
  for (const id of ids) {
    els[id] = document.getElementById(id);
  }
}

/* ── key mapping ───────────────────────────────── */

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
  ["i", "Home"],
]);

/* Map special keys to their WASD kbd label for visual flash */
const flashMap = {
  ArrowUp: "W",
  ArrowDown: "S",
  ArrowLeft: "A",
  ArrowRight: "D",
  Enter: "J",
  " ": "J",
  Backspace: "H",
};

/* ── boot ──────────────────────────────────────── */

async function boot() {
  cacheElements();
  const config = await window.musicos.getConfig();
  state.target = config.target;
  state.syncDir = config.syncDir;
  renderConfig();
  bindEvents();
  await refreshAll();

  /* Polling with backoff: only refreshStatus runs unconditionally.
     Mirror, tracks, and player skip when offline or in mock mode. */
  setInterval(() => {
    if (!state.mockMode) refreshStatus();
  }, 1500);
  setInterval(() => {
    if (state.connected && !state.mockMode) refreshPlayer();
  }, 1500);
  setInterval(() => {
    if (state.connected && !state.mockMode) refreshTracks();
  }, 2600);
  setInterval(() => {
    if (state.connected && !state.mockMode) refreshMirror();
  }, 760);
}

/* ── event bindings ────────────────────────────── */

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

  els.mockModeCheckbox.addEventListener("change", () => {
    state.mockMode = els.mockModeCheckbox.checked;
    if (state.mockMode) {
      showToast("Demo Mode enabled", "success");
      enableMockMode();
    } else {
      showToast("Demo Mode disabled", "info");
      refreshAll();
    }
  });

  /* Target input — guard against Enter+blur double-fire */
  els.targetInput.addEventListener("keydown", async (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      await saveTarget();
      els.targetInput.blur();
    }
  });
  els.targetInput.addEventListener("blur", () => {
    if (!state._savingTarget) saveTarget();
  });

  /* Search — debounced re-render */
  let searchTimer;
  els.searchInput.addEventListener("input", () => {
    state.searchQuery = els.searchInput.value.trim().toLowerCase();
    clearTimeout(searchTimer);
    searchTimer = setTimeout(renderTracks, 150);
  });

  /* Sort */
  els.sortSelect.addEventListener("change", () => {
    state.sortBy = els.sortSelect.value;
    renderTracks();
  });

  /* Now-playing transport */
  els.npPrev.addEventListener("click", () => sendControl("prev"));
  els.npPlayPause.addEventListener("click", () => sendControl("play_pause"));
  els.npNext.addEventListener("click", () => sendControl("next"));

  /* Keyboard forwarding */
  window.addEventListener("keydown", (e) => {
    if (
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLSelectElement
    )
      return;
    const key = e.key.toLowerCase();
    if (key === "r") {
      e.preventDefault();
      refreshAll();
      return;
    }
    const mapped = keyMap.get(key);
    if (mapped) {
      e.preventDefault();
      sendInput(mapped, e.key);
    }
  });

  /* Upload progress from main process */
  window.musicos.onUploadProgress((data) => {
    if (data.phase === "done") {
      els.uploadProgress.hidden = true;
      state.uploadPaused = false;
    } else if (data.phase === "paused") {
      state.uploadPaused = true;
      els.uploadProgress.hidden = false;
      els.uploadProgressText.textContent = data.filename;
      els.uploadProgressFill.style.backgroundColor = "var(--warning)";
      showToast("Upload paused: waiting for connection...", "info");
    } else {
      els.uploadProgress.hidden = false;
      state.uploadPaused = false;
      els.uploadProgressFill.style.backgroundColor = "";
      els.uploadProgressText.textContent = data.filename;
      els.uploadProgressCount.textContent = `${data.current} / ${data.total}`;
      const pct = Math.min((data.current / data.total) * 100, 100);
      els.uploadProgressFill.style.width = `${pct}%`;
    }
  });

  /* Drag and drop folder uploads */
  window.addEventListener("dragover", (e) => {
    e.preventDefault();
    els.dropOverlay.classList.add("active");
  });
  els.dropOverlay.addEventListener("dragleave", (e) => {
    els.dropOverlay.classList.remove("active");
  });
  els.dropOverlay.addEventListener("drop", async (e) => {
    e.preventDefault();
    els.dropOverlay.classList.remove("active");
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const path = e.dataTransfer.files[0].path;
      if (path) {
        await uploadFolder(path);
      }
    }
  });

  /* Mirror interactive controls (taps and swipes) */
  let startX = 0;
  let startY = 0;
  let startTime = 0;

  els.mirror.addEventListener("mousedown", (e) => {
    startX = e.clientX;
    startY = e.clientY;
    startTime = Date.now();
  });

  els.mirror.addEventListener("mouseup", async (e) => {
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    const dt = Date.now() - startTime;

    if (Math.abs(dx) < 10 && Math.abs(dy) < 10) {
      try {
        await postJson("/api/pointer?action=tap", {});
      } catch (err) {
        console.error("tap failed: " + err.message);
      }
    } else if (dt < 500) {
      let dir = "";
      if (Math.abs(dx) > Math.abs(dy)) {
        dir = dx > 0 ? "right" : "left";
      } else {
        dir = dy > 0 ? "down" : "up";
      }
      try {
        await postJson(`/api/pointer?action=swipe&dir=${dir}`, {});
      } catch (err) {
        console.error("swipe failed: " + err.message);
      }
    }
  });
}

/* ── target management ─────────────────────────── */

async function saveTarget() {
  if (state._savingTarget) return;
  const next = els.targetInput.value.trim();
  if (!next || next === state.target) return;
  state._savingTarget = true;
  try {
    const config = await window.musicos.setTarget(next);
    state.target = config.target;
    renderConfig();
    await refreshAll();
  } finally {
    state._savingTarget = false;
  }
}

function renderConfig() {
  els.targetInput.value = state.target;
  els.folderText.textContent = state.syncDir;
}

/* ── refresh cycle ─────────────────────────────── */

async function refreshAll() {
  showToast("Refreshing MusicOS...", "info");
  await Promise.allSettled([
    refreshStatus(),
    refreshSettings(),
    refreshPlayer(),
    refreshTracks(),
    refreshMirror(),
  ]);
}

async function refreshStatus() {
  try {
    const data = await getJson("/api/diagnostics");
    state.diagnostics = data;
    const counts = data.counts || {};
    els.trackCount.textContent = counts.tracks ?? state.tracks.length;
    els.albumCount.textContent = counts.albums ?? "—";
    els.playlistCount.textContent = counts.playlists ?? "—";
    els.pinCount.textContent = counts.favorites ?? "—";
    setConnected(true, "Connected");
    if (state.uploadPaused) {
      state.uploadPaused = false;
      window.musicos.resumeUpload();
      showToast("Connection restored. Resuming upload...", "success");
    }
  } catch {
    setConnected(false, "Waiting for MusicOS");
    if (state.busy && !state.uploadPaused) {
      state.uploadPaused = true;
      window.musicos.pauseUpload();
      showToast("Target offline. Pausing upload...", "warning");
    }
  }
}

async function refreshSettings() {
  try {
    const settings = await getJson("/api/settings");
    state.settings = settings;
    if (document.activeElement !== els.userNameInput)
      els.userNameInput.value = settings.user_name || "";
    if (document.activeElement !== els.deviceNameInput)
      els.deviceNameInput.value = settings.device_name || "";
    if (document.activeElement !== els.audioBackendInput)
      els.audioBackendInput.value = settings.audio_backend || "auto";
    if (document.activeElement !== els.alsaDeviceInput)
      els.alsaDeviceInput.value = settings.alsa_device || "";

    const themeName = (settings.theme || "blue").toLowerCase();
    document.body.classList.remove("dark-theme", "high-contrast");
    if (themeName === "red") {
      document.body.style.setProperty("--accent", "#cf3e52");
      document.body.style.setProperty("--accent-strong", "#a31d30");
      document.body.style.setProperty("--accent-soft", "rgba(207, 62, 82, 0.12)");
      document.body.style.setProperty("--accent-ring", "rgba(207, 62, 82, 0.18)");
    } else if (themeName === "green") {
      document.body.style.setProperty("--accent", "#1e955c");
      document.body.style.setProperty("--accent-strong", "#136b41");
      document.body.style.setProperty("--accent-soft", "rgba(30, 149, 92, 0.12)");
      document.body.style.setProperty("--accent-ring", "rgba(30, 149, 92, 0.18)");
    } else if (themeName === "purple") {
      document.body.style.setProperty("--accent", "#895bc4");
      document.body.style.setProperty("--accent-strong", "#683e9b");
      document.body.style.setProperty("--accent-soft", "rgba(137, 91, 196, 0.12)");
      document.body.style.setProperty("--accent-ring", "rgba(137, 91, 196, 0.18)");
    } else if (themeName === "dark-theme") {
      document.body.classList.add("dark-theme");
      document.body.style.removeProperty("--accent");
    } else if (themeName === "high-contrast") {
      document.body.classList.add("high-contrast");
    } else {
      document.body.style.removeProperty("--accent");
      document.body.style.removeProperty("--accent-strong");
      document.body.style.removeProperty("--accent-soft");
      document.body.style.removeProperty("--accent-ring");
    }
  } catch {
    state.settings = null;
  }
}

async function refreshPlayer() {
  /* Guard against overlapping player refreshes */
  if (state._playerLock) return;
  state._playerLock = true;
  try {
    const data = await getJson("/api/player");
    state.player = data.player;
    renderNowPlaying(data);

    /* If the playing track changed, re-render the list to update the
       playing indicator without re-rendering on every poll. */
    const currentId = data.player?.track?.id ?? null;
    if (currentId !== state._lastPlayingId) {
      state._lastPlayingId = currentId;
      renderTracks();
    }
  } catch {
    state.player = null;
    renderNowPlaying(null);
  } finally {
    state._playerLock = false;
  }
}

async function refreshTracks() {
  try {
    const tracks = await getJson("/api/tracks");
    state.tracks = Array.isArray(tracks) ? tracks : [];
    renderTracks();
  } catch {
    if (!state.connected) renderTracks();
  }
}

async function refreshMirror() {
  try {
    const response = await fetch(
      `${state.target}/api/screenshot.png?t=${Date.now()}`
    );
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const blob = await response.blob();
    const nextUrl = URL.createObjectURL(blob);
    const oldUrl = state.screenshotUrl;
    state.screenshotUrl = nextUrl;
    els.mirror.src = nextUrl;
    els.mirror.style.display = "block";
    els.mirrorEmpty.style.display = "none";
    if (oldUrl) URL.revokeObjectURL(oldUrl);
  } catch {
    if (!state.screenshotUrl) {
      els.mirror.style.display = "none";
      els.mirrorEmpty.style.display = "grid";
    }
  }
}

/* ── track rendering ───────────────────────────── */

function getFilteredTracks() {
  let tracks = [...state.tracks];

  /* Search filter */
  if (state.searchQuery) {
    const q = state.searchQuery;
    tracks = tracks.filter(
      (t) =>
        (t.title || "").toLowerCase().includes(q) ||
        (t.artist || "").toLowerCase().includes(q) ||
        (t.album || "").toLowerCase().includes(q)
    );
  }

  /* Sort */
  const key = state.sortBy;
  tracks.sort((a, b) => {
    if (key === "duration") return (a.duration || 0) - (b.duration || 0);
    const va = (a[key] || "").toLowerCase();
    const vb = (b[key] || "").toLowerCase();
    return va.localeCompare(vb);
  });

  return tracks;
}

function renderTracks() {
  const filtered = getFilteredTracks();
  const playingId = state.player?.track?.id;

  /* Track counter */
  if (state.searchQuery && filtered.length !== state.tracks.length) {
    els.trackCounter.textContent = `${filtered.length} of ${state.tracks.length}`;
  } else {
    els.trackCounter.textContent = state.tracks.length
      ? `${state.tracks.length} tracks`
      : "";
  }

  if (!filtered.length) {
    const msg = state.searchQuery
      ? "No tracks match your search."
      : "Drop music into the sync folder, then upload.";
    els.tracks.innerHTML = `<div class="empty">${msg}</div>`;
    return;
  }

  els.tracks.replaceChildren(
    ...filtered.map((track, index) => {
      const row = document.createElement("div");
      row.className = "track";
      if (track.id === playingId) row.classList.add("track-playing");
      row.style.setProperty("--i", Math.min(index, 15));

      /* Index / playing indicator */
      const number = document.createElement("span");
      number.className = "track-index";
      if (track.id === playingId) {
        number.classList.add("track-index-playing");
        number.innerHTML =
          '<span class="playing-bars"><span></span><span></span><span></span></span>';
      } else {
        number.textContent = String(index + 1).padStart(2, "0");
      }

      /* Cover art */
      const art = document.createElement("img");
      art.className = "track-art";
      art.alt = "";
      art.loading = "lazy";
      if (state.mockMode) {
        art.classList.add("track-art-missing");
      } else {
        art.src = `${state.target}/api/cover-art.png?id=${encodeURIComponent(
          track.id
        )}`;
        art.addEventListener("error", () => {
          art.removeAttribute("src");
          art.classList.add("track-art-missing");
        });
      }

      /* Body (title + meta) */
      const body = document.createElement("span");
      body.className = "track-body";
      const title = document.createElement("span");
      title.className = "track-title";
      title.textContent = track.title || fileTitle(track.path) || "Untitled";
      const meta = document.createElement("span");
      meta.className = "track-meta";
      meta.textContent = [
        track.artist,
        track.album,
        formatDuration(track.duration),
      ]
        .filter(Boolean)
        .join(" · ");
      body.append(title, meta);

      /* Favorite button */
      const fav = document.createElement("button");
      fav.className =
        "track-favorite" + (track.favorited ? " active" : "");
      fav.textContent = track.favorited ? "♥" : "♡";
      fav.title = track.favorited ? "Unfavorite" : "Favorite";
      fav.addEventListener("click", (e) => {
        e.stopPropagation();
        toggleFavorite(track.id);
      });

      /* Actions (revealed on hover via CSS) */
      const actions = document.createElement("span");
      actions.className = "track-actions";
      actions.append(
        trackAction("▶", "Play", () => playTrack(track.id)),
        trackAction("▸▸", "Up Next", () => queueTrack(track.id, "next")),
        trackAction("+", "Add to Queue", () =>
          queueTrack(track.id, "later")
        )
      );

      row.append(number, art, body, fav, actions);
      return row;
    })
  );
}

function trackAction(icon, title, onClick) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = icon;
  button.title = title;
  button.addEventListener("click", (e) => {
    e.stopPropagation();
    onClick();
  });
  return button;
}

/* ── now-playing bar ───────────────────────────── */

function renderNowPlaying(data) {
  const player = data?.player;
  const track = player?.track;

  if (!track) {
    els.nowPlaying.hidden = true;
    return;
  }

  els.nowPlaying.hidden = false;

  /* Cover art */
  const artSrc = state.mockMode ? "" : `${state.target}/api/cover-art.png?id=${encodeURIComponent(
    track.id
  )}`;
  if (state.mockMode) {
    els.npArt.removeAttribute("src");
    els.npArt.classList.add("track-art-missing");
  } else if (els.npArt.src !== artSrc) {
    els.npArt.src = artSrc;
    els.npArt.classList.remove("track-art-missing");
    els.npArt.onerror = () => {
      els.npArt.removeAttribute("src");
      els.npArt.classList.add("track-art-missing");
    };
  }

  /* Track info */
  els.npTitle.textContent =
    track.title || fileTitle(track.path) || "Untitled";
  els.npArtist.textContent =
    [track.artist, track.album].filter(Boolean).join(" · ") || "Unknown";

  /* Progress */
  const pos = player.position || 0;
  const dur = track.duration || 1;
  const pct = Math.min((pos / dur) * 100, 100);
  els.npProgressFill.style.width = `${pct}%`;
  els.npTime.textContent = `${formatTime(pos)} / ${formatTime(dur)}`;

  /* Play / Pause icon */
  els.npPlayPause.textContent = player.is_playing ? "⏸" : "▶";
  els.npPlayPause.title = player.is_playing ? "Pause" : "Play";
}

/* ── actions (all wrapped in try/catch) ────────── */

async function chooseFolder() {
  try {
    const config = await window.musicos.chooseFolder();
    state.syncDir = config.syncDir;
    renderConfig();
    showToast("Sync folder selected.", "success");
  } catch (error) {
    showToast(`Choose folder failed: ${error.message}`, "error");
  }
}

async function uploadFolder(customPath) {
  if (state.busy) return;
  setBusy(true);
  try {
    showToast("Uploading local music...", "info");
    const result = await window.musicos.uploadFolder(customPath);
    const suffix = result.errors?.length
      ? ` (${result.errors.length} errors)`
      : "";
    showToast(
      `Uploaded ${result.uploaded}/${result.found} tracks${suffix}.`,
      result.errors?.length ? "error" : "success"
    );
    await Promise.allSettled([refreshStatus(), refreshTracks()]);
  } catch (error) {
    showToast(`Upload failed: ${error.message}`, "error");
  } finally {
    setBusy(false);
    els.uploadProgress.hidden = true;
  }
}

async function usbSync() {
  try {
    await postJson("/api/sync-usb", {});
    showToast("USB sync requested.", "success");
    await Promise.allSettled([refreshStatus(), refreshTracks()]);
  } catch (error) {
    showToast(`USB sync failed: ${error.message}`, "error");
  }
}

async function saveSettings() {
  try {
    const body = {
      user_name: els.userNameInput.value.trim(),
      device_name: els.deviceNameInput.value.trim(),
      audio_backend: els.audioBackendInput.value,
      alsa_device: els.alsaDeviceInput.value.trim(),
    };
    await postJson("/api/settings", body);
    showToast("Settings saved.", "success");
    await Promise.allSettled([refreshSettings(), refreshStatus()]);
  } catch (error) {
    showToast(`Save settings failed: ${error.message}`, "error");
  }
}

async function scanLibrary() {
  try {
    await postJson("/api/control", { command: "scan_library" });
    showToast("Library scan requested.", "success");
    await Promise.allSettled([refreshStatus(), refreshTracks()]);
  } catch (error) {
    showToast(`Scan failed: ${error.message}`, "error");
  }
}

async function audioTest() {
  try {
    await postJson("/api/control", { command: "audio_test" });
    showToast("Audio test requested.", "success");
    await Promise.allSettled([refreshStatus(), refreshPlayer()]);
  } catch (error) {
    showToast(`Audio test failed: ${error.message}`, "error");
  }
}

async function clearQueue() {
  if (!confirm("Clear the playback queue?")) return;
  try {
    await postJson("/api/control", { command: "queue_clear" });
    showToast("Queue cleared.", "success");
    await refreshPlayer();
  } catch (error) {
    showToast(`Clear queue failed: ${error.message}`, "error");
  }
}

async function showDiagnostics() {
  try {
    const diagnostics = await getJson("/api/diagnostics");
    state.diagnostics = diagnostics;
    const audio = diagnostics.audio || {};
    const firstPcm = audio.pcm_candidates?.[0] || "no PCM";
    showToast(
      `DB: ${diagnostics.db_path || "?"} / PCM: ${firstPcm}`,
      "info"
    );
  } catch (error) {
    showToast(`Diagnostics failed: ${error.message}`, "error");
  }
}

async function sendControl(command) {
  try {
    await postJson("/api/control", { command });
    showToast(`${labelForCommand(command)} sent.`, "success");
    await refreshPlayer();
  } catch (error) {
    showToast(
      `${labelForCommand(command)} failed: ${error.message}`,
      "error"
    );
  }
}

async function playTrack(id) {
  try {
    await postJson("/api/control", { command: "play_track", id });
    showToast("Playing track.", "success");
    await refreshPlayer();
  } catch (error) {
    showToast(`Play failed: ${error.message}`, "error");
  }
}

async function queueTrack(id, position) {
  try {
    await postJson("/api/control", { command: "queue_add", id, position });
    showToast(
      position === "next" ? "Added next." : "Added to queue.",
      "success"
    );
    await refreshPlayer();
  } catch (error) {
    showToast(`Queue failed: ${error.message}`, "error");
  }
}

async function toggleFavorite(id) {
  try {
    await postJson("/api/control", { command: "toggle_favorite", id });
    /* Optimistic local update */
    const track = state.tracks.find((t) => t.id === id);
    if (track) track.favorited = !track.favorited;
    renderTracks();
  } catch (error) {
    showToast(`Favorite failed: ${error.message}`, "error");
  }
}

async function sendInput(key, rawKey) {
  flashKey(rawKey);
  try {
    await fetch(
      `${state.target}/api/input?key=${encodeURIComponent(key)}`,
      { method: "POST" }
    );
  } catch {
    /* silent — key forwarding is fire-and-forget */
  }
}

/* ── network helpers ───────────────────────────── */

async function getJson(path) {
  if (state.mockMode) {
    if (path.startsWith("/api/diagnostics")) {
      return state.diagnostics;
    }
    if (path.startsWith("/api/settings")) {
      return state.settings;
    }
    if (path.startsWith("/api/player")) {
      return { player: state.player };
    }
    if (path.startsWith("/api/tracks")) {
      return state.tracks;
    }
  }
  const response = await fetch(`${state.target}${path}`);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

async function postJson(path, body) {
  if (state.mockMode) {
    if (path.startsWith("/api/control")) {
      const cmd = body.command;
      if (cmd === "play_pause") {
        state.player.is_playing = !state.player.is_playing;
      } else if (cmd === "next") {
        showToast("Skipped to next track", "success");
      } else if (cmd === "prev") {
        showToast("Skipped to previous track", "success");
      } else if (cmd === "play_track") {
        const tr = state.tracks.find(t => t.id === body.id);
        if (tr) {
          state.player.track = tr;
          state.player.is_playing = true;
          state.player.position = 0;
        }
      } else if (cmd === "toggle_favorite") {
        const tr = state.tracks.find(t => t.id === body.id);
        if (tr) {
          tr.favorited = !tr.favorited;
        }
      }
      setTimeout(() => {
        renderTracks();
        renderNowPlaying({ player: state.player });
      }, 50);
      return "ok";
    }
    if (path.startsWith("/api/settings")) {
      state.settings = {
        user_name: body.user_name,
        device_name: body.device_name,
        audio_backend: body.audio_backend,
        alsa_device: body.alsa_device
      };
      return "ok";
    }
    return "ok";
  }
  const response = await fetch(`${state.target}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
}

function enableMockMode() {
  state.connected = true;
  state.diagnostics = {
    db_path: "/var/lib/musicos/media.sqlite3",
    counts: {
      tracks: 4,
      albums: 2,
      playlists: 1,
      favorites: 1
    },
    audio: {
      pcm_candidates: ["default"]
    }
  };
  state.settings = {
    user_name: "Demo User",
    device_name: "MusicOS Simulator",
    audio_backend: "rodio",
    alsa_device: "default"
  };
  state.player = {
    is_playing: true,
    position: 42.0,
    volume: 0.85,
    eq_preset: "Flat",
    shuffle: false,
    loop_mode: "None",
    track: {
      id: "demo-track-1",
      path: "/music/Sample - Dreaming.mp3",
      title: "Dreaming",
      artist: "Stereo Lab",
      album: "Vapor Trails",
      duration: 180
    }
  };
  state.tracks = [
    {
      id: "demo-track-1",
      path: "/music/Sample - Dreaming.mp3",
      title: "Dreaming",
      artist: "Stereo Lab",
      album: "Vapor Trails",
      duration: 180,
      favorited: true
    },
    {
      id: "demo-track-2",
      path: "/music/Sample - Neon.mp3",
      title: "Neon Horizon",
      artist: "Tokyo Synth",
      album: "Metropolis",
      duration: 215,
      favorited: false
    },
    {
      id: "demo-track-3",
      path: "/music/Sample - Rain.mp3",
      title: "Raindrops",
      artist: "Acoustic Duo",
      album: "Unplugged",
      duration: 142,
      favorited: false
    },
    {
      id: "demo-track-4",
      path: "/music/Sample - Beat.mp3",
      title: "Lo-Fi Beats to Code To",
      artist: "Chill Hop",
      album: "Relaxation",
      duration: 320,
      favorited: false
    }
  ];
  
  setConnected(true, "Demo Mode Active");
  els.trackCount.textContent = state.tracks.length;
  els.albumCount.textContent = 2;
  els.playlistCount.textContent = 1;
  els.pinCount.textContent = 1;
  
  els.userNameInput.value = state.settings.user_name;
  els.deviceNameInput.value = state.settings.device_name;
  els.audioBackendInput.value = state.settings.audio_backend;
  els.alsaDeviceInput.value = state.settings.alsa_device;
  
  renderTracks();
  renderNowPlaying({ player: state.player });
  
  els.mirror.src = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='640' height='480' viewBox='0 0 640 480'><rect width='640' height='480' fill='%23111318'/><text x='320' y='240' fill='%23236cdc' font-family='sans-serif' font-size='32' text-anchor='middle'>MusicOS Simulator</text><text x='320' y='280' fill='%2366717d' font-family='sans-serif' font-size='18' text-anchor='middle'>Simulating Now Playing</text></svg>";
  els.mirror.style.display = "block";
  els.mirrorEmpty.style.display = "none";
}

/* ── UI helpers ────────────────────────────────── */

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

/* ── toast system ──────────────────────────────── */

function showToast(message, type = "info") {
  const toast = document.createElement("div");
  toast.className = `toast toast-${type}`;
  toast.textContent = message;
  els.toastContainer.appendChild(toast);

  /* Auto-dismiss after 3 seconds */
  setTimeout(() => {
    toast.classList.add("toast-exit");
    toast.addEventListener("animationend", () => toast.remove());
  }, 3000);

  /* Cap visible toasts at 5 */
  while (els.toastContainer.children.length > 5) {
    els.toastContainer.firstChild.remove();
  }
}

/* ── key flash ─────────────────────────────────── */

function flashKey(key) {
  /* Map arrow keys and special keys to their WASD label */
  const label = flashMap[key] || (key === " " ? "J" : key.toUpperCase());
  document.querySelectorAll("kbd").forEach((kbd) => {
    if (kbd.textContent === label) {
      kbd.classList.add("key-hit");
      setTimeout(() => kbd.classList.remove("key-hit"), 180);
    }
  });
}

/* ── formatters ────────────────────────────────── */

function fileTitle(p) {
  return p
    ?.split(/[\\/]/)
    .pop()
    ?.replace(/\.[^.]+$/, "");
}

function formatDuration(seconds) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

function formatTime(seconds) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

function labelForCommand(command) {
  return command
    .split("_")
    .map((p) => p[0].toUpperCase() + p.slice(1))
    .join(" ");
}

/* ── launch ────────────────────────────────────── */

boot().catch((error) => {
  console.error("Boot failed:", error);
  const container = document.getElementById("toastContainer");
  if (container) {
    const toast = document.createElement("div");
    toast.className = "toast toast-error";
    toast.textContent = `Sync app failed: ${error.message}`;
    container.appendChild(toast);
  }
});
