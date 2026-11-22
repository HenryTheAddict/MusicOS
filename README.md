# musicOS

musicOS is a React + Tauri 2 desktop music player with DJ mode and library
management, an embedded handheld OS for the Anbernic RG35XX 2024 / Allwinner
H700, and companion mobile apps.

Desktop documentation: [User guide](docs/guide/README.md) · [Architecture and verification](docs/desktop-architecture.md).

## Download

Get the [0.1.1 beta (v0.1.1)](https://github.com/HenryTheAddict/MusicOS/releases/tag/v0.1.1):

| Platform | Download |
| --- | --- |
| macOS 14 Sonoma or later, Apple Silicon | [DMG](https://github.com/HenryTheAddict/MusicOS/releases/download/v0.1.1/musicOS_0.1.1_darwin-aarch64.dmg) |
| Windows x64 | [Installer](https://github.com/HenryTheAddict/MusicOS/releases/download/v0.1.1/musicOS_0.1.1_windows-x86_64-setup.exe) |
| Linux x86_64 | [AppImage](https://github.com/HenryTheAddict/MusicOS/releases/download/v0.1.1/musicOS_0.1.1_linux-x86_64.AppImage) |

The macOS build needs macOS 14 Sonoma or later and is Apple Silicon only for now. Intel Mac and Arch packages
are being added separately.

Supported release builds offer manual update checks in **Settings → General →
Software updates**. See [Desktop releases and in-app updates](docs/desktop-releases.md)
for supported formats and update behavior.

## React desktop app

The default desktop is now React + TypeScript in Tauri 2, backed by the existing
Rust library, audio, metadata, and device services without GPUI. It uses the
Sketch-based visual design and bundled Figtree fonts, native macOS window
controls, and ordinary overflow scrolling.

```sh
./run.sh                 # install JS dependencies if needed; launch Tauri dev
npm run build            # build frontend, preview sidecar, and desktop binary
npm run package -- --bundles app  # macOS app bundle
```

Windows entry point: `scripts/run-desktop.ps1`. Development requires Node 22.12+
(or Node 24), Rust, and the platform's Tauri prerequisites. A browser-only
`npm run dev` preview has no filesystem or playback access.

See [Desktop releases and in-app updates](docs/desktop-releases.md) for signed
packages and GitHub Releases setup.

Library includes Songs, Albums, Artists, Genres, album/artist details, native
folder import, managed copies, reference folders, rescan, and metadata review.
Discover has been removed from the desktop interface. Settings has Appearance,
Audio, Library, Integrations and General tabs. Library settings includes batch
metadata editing and folder management; appearance preferences remain Coverflow
Colors, Your name, and Interface size. Devices connects to the existing API,
launches the bundled embedded preview, and reviews a folder before syncing.

The app reuses the Rust desktop configuration's database and media paths.
Compatible C++ appearance preferences migrate on first launch. Originals remain
in place during reference imports; managed import explicitly copies files.
See [desktop architecture and usage](docs/react-desktop.md) and the
[current verification record](docs/react-desktop-qa.md) for exact behavior and
remaining acceptance checks.

```sh
npm run test:desktop-core
npm run test:tauri
npm run test:ui
npx playwright install chromium
npm run dev             # separate terminal for browser integration tests
npm run test:e2e
```

## Legacy desktop implementations

Both previous implementations remain available explicitly:

```sh
scripts/run-desktop-gpui.sh  # GPUI Control Center
npm run desktop:legacy      # GPUI directly
npm run package:legacy      # legacy cargo-packager workflow
```

PowerShell GPUI launcher: `scripts/run-desktop-gpui.ps1`.
The embedded musicOS runtime and its `musicos-mac` preview remain unchanged by
the React rewrite. To launch that runtime directly:

```sh
cargo run --bin musicos-mac --features desktop-app,desktop-audio
```

## Android offline player

The native Android app copies selected music, artwork and collection sequences from the Mac over encrypted Wi-Fi, then plays independently offline. Mac **Devices** handles sharing and phone approval. The phone provides a background audio session, media notifications, a persistent queue, lock-screen/headset controls and the supplied Sketch/Figtree presentation.

See [Android setup, build and transfer contract](docs/android-offline-sync.md). This is separate from the embedded musicOS OS image and desktop runtime/model bundle.

## Embedded and shared capabilities

- Rust `musicos-init` binary intended to install as `/sbin/init`.
- PID 1 setup for `/proc`, `/sys`, `/dev`, `/run`, and `/var/lib/musicos`.
- Classic-iPod-inspired setup, home, music, now-playing, settings, status, and
  overlay screens with restrained accent themes.
- Raster Gothic font rendering from the bundled font archive.
- `640x480` and `1280x960` logical display modes.
- Linux framebuffer rendering with in-memory fallback for host checks.
- evdev gamepad/button input and Apple TV-style swipe/tap navigation.
- SQLite media indexer for local MicroSD-style media folders.
- USB sync path that indexes mounted media and a Settings `Get Update` action
  that installs `musicos-init` updates from USB storage.
- Native desktop app target for Windows/macOS/Linux UI testing.
- Legacy GPUI `MusicOS Control Center` for Windows, macOS, and Linux, with an
  iTunes 9-inspired library shell, virtualized song rows, Cover Flow, playback
  chrome, device operations, diagnostics, and local emulator controls.
- Framework-independent Rust services for managed imports, reference sources,
  sync, transcoding, playlist reconciliation, persistence, and runtime process
  management.
- `musicos` CLI for library scans, stats, playlists, sync dry-run planning,
  USB update staging/install checks, settings, devices, and doctor output.
- Smart dynamic mixes, playback history, queue maintenance, seek controls,
  sleep timer commands, motion/performance settings, and cover-art cache tools.
- Ordered playlist tracks, sync dry-run/retry tools, database repair, changed
  versus full library rescans, and frame timing diagnostics.
- Buildroot external tree for `musicos-qemu` and `musicos-rg35xx`.
- QEMU launch script using `qemu-system-aarch64 -M virt -cpu cortex-a53`.

## Quick checks

```sh
scripts/check-host.sh
```

On Windows PowerShell:

```powershell
.\scripts\check-host.ps1
```

## musicOS CLI

Run the CLI through npm or Cargo:

```sh
npm run cli -- --help
cargo run --bin musicos --features desktop-app,desktop-audio -- --help
```

Examples:

```sh
npm run cli -- scan
npm run cli -- scan --all
npm run cli -- library list
npm run cli -- library stats --json
npm run cli -- library add ./music
npm run cli -- playlists list
npm run cli -- playlists create "Roadtrip"
npm run cli -- playlists add-track 1 42
npm run cli -- devices list
npm run cli -- sync plan desktop --json
npm run cli -- sync start desktop --dry-run
npm run cli -- updates scan
npm run cli -- updates install
npm run cli -- settings get --json
npm run cli -- settings set user-name "Henry"
npm run cli -- settings set device-name "MusicOS"
npm run cli -- settings set audio-backend alsa
npm run cli -- settings set alsa-device plughw:0,0
npm run cli -- settings set theme red
npm run cli -- settings set eq bass
npm run cli -- doctor --json
```

The CLI reuses the Rust SQLite, media indexing, preferences, playlist, USB
update, and companion upload paths. `sync start --dry-run` never writes to a
device. `sync start desktop` uploads files from `MUSICOS_SYNC_DIR` to
`MUSICOS_SYNC_TARGET` using the same `/api/upload` headers as the Control Center,
then triggers one `/api/control` library scan. You can also pass an explicit
`http://host:port` target as the device id for a reachable musicOS companion API.

### Agent sync CLI

`musicos-agent` is the non-interactive sync interface for scripts and automation
agents. It shares the Control Center's file validation, transcoding,
SHA-256 hashing, upload, skip detection, and library scan implementation.
The command is now a Rust binary backed by `musicos-desktop`; the npm shortcut
retains the existing command line, JSON/JSON-lines output, and exit-code contract.

```sh
npm run agent -- device status --target http://192.168.7.2:8080 --json
npm run agent -- devices list --target http://192.168.7.2:8080 --json
npm run agent -- sync plan --source ~/Music --codec vorbis --bitrate 96k --json
npm run agent -- sync start --source ~/Music --target http://192.168.7.2:8080 \
  --codec vorbis --bitrate 96k --sample-rate 48000 --json-lines
```

Use repeated `--file <path>` options for a scoped sync. `--json` emits one final
document; `--json-lines` emits progress events as JSON Lines. Exit codes are `0`
for success, `2` for invalid arguments, `3` when the configured device is
unreachable, and `4` when a sync completes with failures. Device listing probes
the configured target; it does not scan the local network.

Run the cross-platform runtime performance acceptance test with:

```sh
npm run perf:smoke
```

It seeds 10,000 WAV tracks under `out/perf`, starts the desktop runtime, checks
SQL-backed paging and search latency, screenshot availability, input latency,
scan completion, and frame timing at the RG35XX-style `640x480` resolution. It writes
`out/perf/runtime-performance-smoke.json`. It fails nonzero if first screenshot,
frame p95, core API p95, search p95, scan completion, or indexed track count
misses the acceptance thresholds. Override the seed with
`MUSICOS_PERF_TRACKS`, the resolution with `MUSICOS_PERF_RESOLUTION`, or skip
the build with `MUSICOS_PERF_SKIP_BUILD=1`.

Useful runtime APIs:

- `GET /api/library` returns counts, playback, queue, playlists, diagnostics
  summary, legacy `scan_status`, structured `scan`, and build `version`.
- `GET/POST /api/library/sources` lists or adds managed/reference roots;
  `PATCH/DELETE /api/library/sources/:id` updates or removes a reference, and
  `POST /api/library/sources/:id/rescan` requests a refresh.
- `GET /api/tracks?query=&limit=&offset=&sort=&filter=&artist=&album=&playlist_id=`
  returns the compatible track array with optional filtering.
- `GET /api/v2/tracks`, `/api/v2/albums`, `/api/v2/artists`, and
  `/api/v2/genres` provide SQL-backed paginated library browsing with stable
  track identities and content hashes.
- `PATCH /api/v2/tracks/metadata` transactionally writes common audio tags and
  refreshes the indexed metadata.
- `GET /api/v2/playlists/manifest` and `POST /api/v2/playlists/reconcile`
  expose revisioned, hash-based playlist manifests and three-way conflict
  detection for desktop/device synchronization.
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
- `POST /api/sync-usb` scans configured/removable USB roots, indexes USB media,
  stages musicOS update payloads, and installs update targets when present.
- `POST /api/updates/install` installs an already-staged musicOS binary or
  system image update without requiring another USB scan.
- `PUT /api/playlists/:id/tracks/reorder` with `{ "track_ids": [...] }`
  persists manual playlist order.
- `POST /api/control` accepts playback, queue, playlist, scan, seek, sleep
  timer, and smart-mix commands and returns an acknowledged command result with
  the current player revision.

GitHub Actions CI is defined in `.github/workflows/ci.yml`. The legacy native
Control Center is checked, tested, and release-built on macOS, Windows, and
Linux. React and Tauri builds and tests also run on all three platforms, with
browser integration checks on Linux and additional Windows workspace and
runtime performance checks.

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

For a USB update, put a new musicOS image at one of these paths on the USB
drive: `musicos-update/musicos-knulli-ui.squashfs`,
`musicos-update/musicos-knulli-root.squashfs`, `MusicOS/update/knulli`, or
`knulli`. The updater also still accepts raw `musicos-init` ELF binaries at
`musicos-update/musicos-init`, `MusicOS/update/musicos-init`, or `musicos-init`.
On device, open Settings, choose `Get Update`, then restart after the status
screen reports the update was installed.

## QEMU build and run

```sh
scripts/build-qemu.sh
scripts/run-qemu.sh
```

To run QEMU and the legacy native GPUI Control Center together:

```sh
scripts/run-qemu-sync.sh
```

`scripts/run-qemu.sh` defaults to `MUSICOS_QEMU_NET=sync`, which forwards the
guest musicOS API on port `8080` to `http://127.0.0.1:18080` for the Control
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

To flash the current KNULLI-backed musicOS image to an attached RG35XX SD card:

```sh
scripts/flash-knulli-music-os.sh /dev/diskN
```

The script refuses non-removable disks and whole-drive sizes above 128GB, then
copies the newest `musicos-knulli-ui.squashfs` payload into `/boot/knulli`.
Music is not bundled by default; set `MUSICOS_SAMPLE_DIR=/path/to/music` only
when you explicitly want to copy your own test tracks to the FAT partition.

Use the half-scale mode with:

```sh
MUSICOS_RESOLUTION=640x480 scripts/run-qemu.sh
```

The QEMU target intentionally uses generic `virt` hardware with virtio GPU,
keyboard, tablet, and block devices. It does not pretend to emulate the RG35XX
H700 display pipeline exactly.

See the [embedded QEMU boot and playback baseline](docs/embedded-qemu-baseline.md)
for a headless smoke test with serial logs, framebuffer captures, and recorded
audio.

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

