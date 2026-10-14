#!/usr/bin/env node

const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { performance } = require("node:perf_hooks");

const root = path.resolve(__dirname, "..");
const outDir = path.join(root, "out", "perf");
const musicDir = path.join(outDir, "music-smoke");
const dbPath = path.join(outDir, "perf-smoke.sqlite3");
const reportPath = path.join(outDir, "runtime-performance-smoke.json");
const trackCount = Number(process.env.MUSICOS_PERF_TRACKS || 10_000);
const samples = Number(process.env.MUSICOS_PERF_SAMPLES || 30);
const port = Number(process.env.MUSICOS_PERF_PORT || 18_180);
const timeoutMs = Number(process.env.MUSICOS_PERF_TIMEOUT_MS || 120_000);
const baseUrl = `http://127.0.0.1:${port}`;
const executable = path.join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "musicos-mac.exe" : "musicos-mac",
);

function wavBytes() {
  const sampleRate = 8_000;
  const sampleCount = 400;
  const dataLength = sampleCount * 2;
  const value = Buffer.alloc(44 + dataLength);
  value.write("RIFF", 0);
  value.writeUInt32LE(36 + dataLength, 4);
  value.write("WAVEfmt ", 8);
  value.writeUInt32LE(16, 16);
  value.writeUInt16LE(1, 20);
  value.writeUInt16LE(1, 22);
  value.writeUInt32LE(sampleRate, 24);
  value.writeUInt32LE(sampleRate * 2, 28);
  value.writeUInt16LE(2, 32);
  value.writeUInt16LE(16, 34);
  value.write("data", 36);
  value.writeUInt32LE(dataLength, 40);
  for (let index = 0; index < sampleCount; index += 1) {
    value.writeInt16LE(index % 2 ? -10_000 : 10_000, 44 + index * 2);
  }
  return value;
}

function percentile(values, amount) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * amount) - 1)] || 0;
}

async function waitFor(label, probe) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const result = await probe();
      if (result) return result;
    } catch {
      // The runtime may still be starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function getJson(route) {
  const response = await fetch(`${baseUrl}${route}`);
  if (!response.ok) throw new Error(`${route} returned ${response.status}`);
  return response.json();
}

async function measure(route, count = samples) {
  const timings = [];
  let bytes = 0;
  for (let index = 0; index < count; index += 1) {
    const started = performance.now();
    const response = await fetch(`${baseUrl}${route}`);
    const body = await response.arrayBuffer();
    timings.push(performance.now() - started);
    bytes = body.byteLength;
    if (!response.ok) throw new Error(`${route} returned ${response.status}`);
  }
  return {
    path: route,
    count,
    min_ms: Math.min(...timings),
    average_ms: timings.reduce((sum, value) => sum + value, 0) / timings.length,
    p95_ms: percentile(timings, 0.95),
    max_ms: Math.max(...timings),
    bytes,
  };
}

async function seedLibrary() {
  fs.rmSync(musicDir, { recursive: true, force: true });
  fs.mkdirSync(musicDir, { recursive: true });
  const audio = wavBytes();
  for (let index = 1; index <= trackCount; index += 1) {
    const artist = `Perf Artist ${String((index % 100) + 1).padStart(3, "0")}`;
    const album = `Perf Album ${String((index % 500) + 1).padStart(3, "0")}`;
    const albumDir = path.join(musicDir, artist, album);
    fs.mkdirSync(albumDir, { recursive: true });
    fs.writeFileSync(
      path.join(albumDir, `${String(index).padStart(5, "0")} - Perf Track ${index}.wav`),
      audio,
    );
  }
}

async function main() {
  fs.mkdirSync(outDir, { recursive: true });
  for (const suffix of ["", "-shm", "-wal"]) fs.rmSync(`${dbPath}${suffix}`, { force: true });
  await seedLibrary();

  if (process.env.MUSICOS_PERF_SKIP_BUILD !== "1") {
    const build = spawnSync(
      "cargo",
      ["build", "--bin", "musicos-mac", "--features", "desktop-app,desktop-audio"],
      { cwd: root, stdio: "inherit" },
    );
    if (build.status !== 0) throw new Error("Performance runtime build failed");
  }

  const child = spawn(executable, [], {
    cwd: root,
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      MUSICOS_AUDIO_BACKEND: "none",
      MUSICOS_COMPANION_PORT: String(port),
      MUSICOS_MEDIA_DB: dbPath,
      MUSICOS_UPLOAD_DIR: musicDir,
      MUSICOS_SYNC_DIR: musicDir,
      MUSICOS_REINDEX_ALL: "1",
      MUSICOS_SETTINGS_PATH: path.join(outDir, "settings-smoke.json"),
      MUSICOS_RESOLUTION: process.env.MUSICOS_PERF_RESOLUTION || "640x480",
    },
  });
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr = `${stderr}${chunk}`.slice(-8_000);
  });

  const started = performance.now();
  try {
    await waitFor("library API", () => getJson("/api/library"));
    const readyMs = performance.now() - started;
    const screenshotBytes = await waitFor("first screenshot", async () => {
      const response = await fetch(`${baseUrl}/api/screenshot.png`);
      const bytes = (await response.arrayBuffer()).byteLength;
      return response.ok && bytes > 1_000 ? bytes : 0;
    });
    const warmDisplayMs = performance.now() - started;
    const library = await waitFor("10,000-track scan", async () => {
      const state = await getJson("/api/library");
      return state.counts?.tracks >= trackCount ? state : null;
    });

    const api = await Promise.all([
      measure("/api/player"),
      measure("/api/diagnostics"),
      measure("/api/v2/tracks?limit=50&offset=0&sort=title"),
    ]);
    const search = await measure("/api/v2/tracks?query=Perf%20Track%2099&limit=50", samples);
    const inputs = [];
    for (const key of ["Down", "Up", "Right", "Left"]) {
      const inputStarted = performance.now();
      const response = await fetch(`${baseUrl}/api/input?key=${key}`, { method: "POST" });
      if (!response.ok) throw new Error(`input returned ${response.status}`);
      inputs.push(performance.now() - inputStarted);
    }
    const frameValues = [];
    for (let index = 0; index < samples; index += 1) {
      const diagnostics = await getJson("/api/diagnostics");
      if (Number.isFinite(diagnostics.frame_ms)) frameValues.push(diagnostics.frame_ms);
    }

    const report = {
      generated_at: new Date().toISOString(),
      platform: process.platform,
      track_count: trackCount,
      ready_ms: readyMs,
      warm_display_ms: warmDisplayMs,
      first_screenshot_bytes: screenshotBytes,
      counts: library.counts,
      api,
      search,
      input_p95_ms: percentile(inputs, 0.95),
      frame_p95_ms: percentile(frameValues, 0.95),
    };
    fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

    const failures = [];
    if (warmDisplayMs > 1_500) failures.push(`warm display ${warmDisplayMs.toFixed(1)}ms > 1500ms`);
    for (const result of api) {
      if (result.p95_ms > 30) failures.push(`${result.path} p95 ${result.p95_ms.toFixed(1)}ms > 30ms`);
    }
    if (search.p95_ms > 150) failures.push(`search p95 ${search.p95_ms.toFixed(1)}ms > 150ms`);
    if (report.input_p95_ms > 50) failures.push(`input p95 ${report.input_p95_ms.toFixed(1)}ms > 50ms`);
    if (report.frame_p95_ms > 20) failures.push(`frame p95 ${report.frame_p95_ms.toFixed(1)}ms > 20ms`);
    if (failures.length) throw new Error(failures.join("; "));
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  } catch (error) {
    if (stderr) process.stderr.write(stderr);
    throw error;
  } finally {
    child.kill();
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
