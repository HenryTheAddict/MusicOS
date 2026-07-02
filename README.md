# MusicOS

Minimal Rust-first embedded Linux shell for low-energy music hardware. The first
target is Anbernic RG35XX 2024 / Allwinner H700, with a real QEMU aarch64 boot
target and a desktop preview loop for Windows, macOS, and Linux development.

## What Is Implemented

- Rust `musicos-init` binary intended to install as `/sbin/init`.
- PID 1 setup for `/proc`, `/sys`, `/dev`, `/run`, and `/var/lib/musicos`.
- Figma-matched setup, hello, home, apps, music, now-playing, settings, status,
  and test-complete screens.
- Raster Gothic font rendering from the bundled font archive.
- `640x480` and `1280x960` logical display modes.
- Linux framebuffer rendering with in-memory fallback for host checks.
- evdev gamepad/button input and Apple TV-style swipe/tap navigation.
- SQLite media indexer for local MicroSD-style media folders.
- USB sync path that indexes mounted media and a Settings `Get Update` action
  that installs `musicos-init` updates from USB storage.
- Native desktop app target for Windows/macOS/Linux UI testing.
- Electron `MusicOS Control Center` for the Windows/macOS/Linux desktop
  emulator, device mirror, library sync, playlists, diagnostics, logs, and
  local emulator controls.
- Smart dynamic mixes, playback history, queue maintenance, seek controls,
  sleep timer commands, motion/performance settings, and cover-art cache tools.
- Ordered playlist tracks, sync dry-run/retry tools, database repair, changed
  versus full library rescans, and frame timing diagnostics.
- Buildroot external tree for `musicos-qemu` and `musicos-rg35xx`.
- QEMU launch script using `qemu-system-aarch64 -M virt -cpu cortex-a53`.

## Quick Checks

```sh
scripts/check-host.sh
```

On Windows PowerShell:

```powershell
.\scripts\check-host.ps1
```

For the normal desktop development loop, run the emulator and Control Center
together:

```sh
scripts/run-desktop.sh
```

On Windows PowerShell:

```powershell
.\scripts\run-desktop.ps1
```

That launches the Electron Control Center with one shared SQLite database, media
folder, and companion API target. Use the emulator controls inside the app to
build and start the desktop MusicOS runtime:

- API: `http://127.0.0.1:18080`
- DB: `out/desktop-media.sqlite3`
- Upload folder: `./music`

Override with `MUSICOS_COMPANION_PORT`, `MUSICOS_MEDIA_DB`,
`MUSICOS_SYNC_TARGET`, or `MUSICOS_SYNC_DIR` when needed.

For the desktop test app by itself:

```sh
cargo run --bin musicos-mac --features desktop-app,desktop-audio
```

The desktop target stores its default test database at `out/mac-media.sqlite3` and
serves the device API on `http://127.0.0.1:18080`. Override that with
`MUSICOS_MEDIA_DB` or `MUSICOS_COMPANION_PORT` when needed.

For the Electron Control Center by itself:

```sh
npm run sync:electron
```

The Control Center starts `target/debug/musicos-mac.exe` on Windows when you use
its emulator controls. If the binary is missing, it runs
`cargo build --bin musicos-mac --features desktop-app,desktop-audio`, streams the
build output into Logs, and then launches the emulator. The app targets
`http://127.0.0.1:18080` by default. Use
`MUSICOS_SYNC_TARGET=http://192.168.7.2:8080` for a USB-networked device, and
`MUSICOS_SYNC_DIR=/path/to/music` to choose the folder uploaded by the app.
Folder sync preserves relative artist/album paths, skips unchanged files by
size/checksum, triggers one library scan after the batch, and can export a JSON
diagnostics bundle from the Logs view. The Device view also exposes library
backup/restore, playlist import/export, sync history, cover-art cache clearing,
database repair, changed/full rescans, and a perf-smoke launcher. The Library
view can dry-run a sync and retry only failed files from the last sync attempt.
Keyboard controls are `W/A/S/D` for direction, `J` for confirm, `H` for back,
`U` for context, and `I` for home.

Windows PowerShell equivalent:

```powershell
$env:MUSICOS_SYNC_TARGET = "http://192.168.7.2:8080"
$env:MUSICOS_SYNC_DIR = "C:\Music"
npm run sync:electron
```

You can also use the npm shortcuts on any host with Rust and Node installed:

```sh
npm run check
npm run build
npm run desktop
```

On Windows, run the local runtime performance smoke test with:

```powershell
npm run perf:smoke
```

It seeds a small WAV library under `out/perf`, starts the desktop runtime,
checks API latency, screenshot availability, input latency, scan completion,
frame timing at the RG35XX-style `640x480` resolution, and writes
`out/perf/runtime-performance-smoke.json`. It fails nonzero if first screenshot,
frame p95, core API p95, scan completion, or indexed track count misses the
default thresholds. Pass `-Resolution 1280x960` to stress the HD desktop path.

Useful runtime APIs:

- `GET /api/library` returns counts, playback, queue, playlists, diagnostics
  summary, legacy `scan_status`, structured `scan`, and build `version`.
- `GET /api/tracks?query=&limit=&offset=&sort=&filter=&artist=&album=&playlist_id=`
  returns the compatible track array with optional filtering.
- `GET /api/mixes` and `GET /api/mixes/:id/tracks` expose dynamic mixes such as
  Favorites, Recently Added, Most Played, Never Played, Long Tracks, Shuffle
  All, and Random Album.
- `GET /api/history?limit=&offset=` returns recently played tracks.
- `GET /api/performance` returns frame timing, motion mode, scan state, and
  cover-art cache counters.
- `GET /api/logs?limit=120` returns recent runtime events for diagnostics export
  and the Control Center Logs view.
- `POST /api/library/cover-art-cache/clear` clears persisted and in-memory
  cover-art cache state so art can be rebuilt.
- `POST /api/library/repair` runs SQLite integrity checks, missing-file cleanup,
  orphan playlist cleanup, playlist position repair, and a WAL checkpoint.
- `POST /api/library/rescan` with `{ "mode": "changed" | "full" }` starts a
  bounded scan using changed-file skips or a full metadata/art refresh.
- `PUT /api/playlists/:id/tracks/reorder` with `{ "track_ids": [...] }`
  persists manual playlist order.
- `POST /api/control` accepts playback, queue, playlist, scan, seek, sleep
  timer, and smart-mix commands.

GitHub Actions CI is defined in `.github/workflows/ci.yml` and runs the Windows
Rust check/test, Electron JavaScript syntax checks, and desktop binary build.

For static visual snapshots:

```sh
MUSICOS_SNAPSHOT_SCREEN=home MUSICOS_RESOLUTION=640x480 cargo run --bin musicos-snapshot -- out/home-640.bmp
MUSICOS_SNAPSHOT_SCREEN=apps cargo run --bin musicos-snapshot -- out/apps.bmp
MUSICOS_SNAPSHOT_SCREEN=about cargo run --bin musicos-snapshot -- out/about.bmp
```

Set `MUSICOS_TIME_TEXT="6:48 PM"` when comparing snapshots to the Figma
exports so the live clock does not pollute visual diffs.

To test USB media/update behavior on a host:

```sh
MUSICOS_USB_ROOTS=/path/to/usb MUSICOS_UPDATE_STAGING=out/staged-update cargo test --workspace --features desktop-app,desktop-audio
```

On Windows PowerShell:

```powershell
$env:MUSICOS_USB_ROOTS = "E:\"
$env:MUSICOS_UPDATE_STAGING = "out\staged-update"
cargo test --workspace --features desktop-app,desktop-audio
```

For a USB update, put a new MusicOS image at one of these paths on the USB
drive: `musicos-update/musicos-knulli-ui.squashfs`,
`musicos-update/musicos-knulli-root.squashfs`, `MusicOS/update/knulli`, or
`knulli`. The updater also still accepts raw `musicos-init` ELF binaries at
`musicos-update/musicos-init`, `MusicOS/update/musicos-init`, or `musicos-init`.
On device, open Settings, choose `Get Update`, then restart after the status
screen reports the update was installed.

## QEMU Build And Run

```sh
scripts/build-qemu.sh
scripts/run-qemu.sh
```

To run QEMU and the Electron Control Center together:

```sh
scripts/run-qemu-sync.sh
```

`scripts/run-qemu.sh` defaults to `MUSICOS_QEMU_NET=sync`, which forwards the
guest MusicOS API on port `8080` to `http://127.0.0.1:18080` for the Control
Center.
Use `MUSICOS_QEMU_NET=none scripts/run-qemu.sh` for the fully offline QEMU run,
or `MUSICOS_QEMU_HOST_PORT=18081 scripts/run-qemu-sync.sh` if port `18080` is
already in use.

On macOS and Windows, Buildroot needs GNU/Linux host tooling. Use Docker Desktop
or a Linux/WSL environment for firmware images:

```sh
scripts/build-qemu-container.sh
scripts/run-qemu.sh
```

Windows PowerShell Docker wrapper:

```powershell
.\scripts\build-qemu-container.ps1
```

For the RG35XX 2024 image on macOS or Windows, use the matching container
wrapper:

```sh
scripts/build-rg35xx-container.sh
```

Windows PowerShell:

```powershell
.\scripts\build-rg35xx-container.ps1
```

The container wrapper builds a cached local image named
`musicos-buildroot:24.04` so subsequent retries do not reinstall Ubuntu build
dependencies.

To flash the current KNULLI-backed MusicOS image to an attached RG35XX SD card:

```sh
scripts/flash-knulli-music-os.sh /dev/diskN
```

The script refuses non-removable disks and whole-drive sizes above 128GB, then
copies the newest `musicos-knulli-ui.squashfs` payload into `/boot/knulli` and
copies sample music to the FAT partition.

Use the half-scale mode with:

```sh
MUSICOS_RESOLUTION=640x480 scripts/run-qemu.sh
```

The QEMU target intentionally uses generic `virt` hardware with virtio GPU,
keyboard, tablet, and block devices. It does not pretend to emulate the RG35XX
H700 display pipeline exactly.

## RG35XX Build

```sh
scripts/build-rg35xx.sh
```

The RG35XX defconfig is a mainline-kernel starting point. Real hardware display
and input bring-up may need the smallest available H700/RG35XX device-tree and
driver patch set.

## Audio Features

Desktop builds use `desktop-audio`, which enables Rodio for the emulator and
Control Center workflow. KNULLI/RG35XX payload builds use `pid1,device-audio`,
which enables the ALSA probing and playback path used on device. The legacy
`audio` feature remains available as a compatibility umbrella for both audio
backends.
