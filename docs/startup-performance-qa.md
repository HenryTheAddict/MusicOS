# Startup and playback UI performance — October 16, 2026

Scope: existing startup and rendering paths. Your Radio, Hidden Gems, and adaptive skipping remain deferred.

## Changes

- Persist artwork validation by path, file size, nanosecond modification time, and validation version. Reopening MusicOS reuses successful validation without decoding each cover. Replacements and invalid images still undergo bounded decoding. A rejected replacement retains valid canonical artwork. Durable validation metadata is capped at 16,384 entries; the existing process cache remains capped at 4,096.
- Commit canonical artwork updates together and skip unchanged artwork writes. Index tracks for artwork assignment instead of scanning the library for every album.
- Register each canonical asset path once per process, retaining registration retries on errors.
- Load Settings, library tools, collection management, Sparklist creation, audio analysis, Devices, and SuperCommand when needed, with Suspense handling for navigation and dialogs.
- Memoize playlist artwork and sidebar counts/durations across playback-position updates. Index artist/album membership and use the existing track index for album and playlist lookup.

## Measurements

The release-mode `library-startup-qa` example snapshots the real SQLite database to a temporary database. Original media is read only. Two separate child processes use the same temporary database: the first populates validation, and the second demonstrates reuse across a process restart. Seven samples are collected per process. This isolates the backend library phase; it is **not** a measurement of process launch to interactive native window.

Library: 507 tracks, 52 albums, 417,124-byte serialized payload.

| Measurement | Before | After |
| --- | ---: | ---: |
| First-process artwork validation without a durable cache | 140.77 ms | 155.84 ms |
| Artwork validation in a fresh process after cache population | No persistent cache | 1.43 ms |
| Complete sampled library phase in that cached fresh process | — | 25.75 ms |

First-ever validation is still required; the optimization targets subsequent launches. Measurements ran alongside verification work, so the uncached difference is not an isolated regression benchmark. OS file caches also influence grouping and source audits. Do not interpret the library-phase measurements as a total startup-speed percentage.

Initial production JavaScript previously measured 758.48 kB (210.25 kB gzip). The initial static chunk set now totals 688.31 kB, about 9% less raw JavaScript. On-demand screen chunks are still included in the app and require no network connection.

## Verification

- 115 native app unit tests passed, including durable validation after connection restart, corrupt/replaced artwork, validation-version invalidation, canonical artwork consistency, and user overrides.
- 70 frontend unit tests passed.
- Production TypeScript and Vite builds passed.
- SuperCommand deferred loading, search, collection creation, and Escape passed in the browser fixture.
- 12 distinct browser checks passed: artwork geometry/keyboard menus, flat Settings, collection rules, metadata review/Undo, all 40 management size/scale/palette cases, Sparklist length/layout/persistence, SuperCommand, and focused deferred-screen navigation during playback.
- Two broad sweeps timed out in the initial run, which overlapped source edits and a separate Playwright invocation sharing the artifact directory. Both passed with unchanged source and separate artifacts on rerun: management matrix 3.6 minutes, Sparklist matrix 57.5 seconds (60-second test limit). The original timeouts are retained in `/tmp/musicos-startup-navigation.log`; they are not counted as passes.
- No changes to the audio processor, source files, collection identities, saved sequences, or depth model assets.

## Delivery

- Local ad-hoc signed Apple Silicon app: `target/release/bundle/macos/MusicOS.app`, 98.41 MiB.
- Executable SHA-256: `1f05d219c1af3bcf1221e383daf5b9b9ee5ba404082948b7c4f6c2484e80030e`.
- Deep/strict signature verification passed. The source sidecar and ONNX runtime were preserved (`6b1fa66d…80e1d`, `ff52828c…34920`). The bundled sidecar was re-signed with the app; its resulting SHA-256 is `549c3b19a6b9bd6273163c12e0c102c54b082d74029da0c9943abfc84340a244`. The bundled ONNX runtime remains byte-identical.
- Loaded while paused. Restored Picture in My Mind to 91.2 seconds; playback remains paused. Native library shows 507 songs and 11 favorites, with the same saved collections.
- Packaged Settings and Library tools loaded on demand. Native screenshot inspected after returning to the current album; adaptive artwork colors, 125% interface size, and enabled depth/parallax preferences remain intact.
- Native 120 Hz frame pacing and full process-to-interactive startup remain unmeasured in this pass. Windows/Linux remain unverified.

Logs: `/tmp/musicos-startup-before.json`, `/tmp/musicos-startup-after.json`, `/tmp/musicos-startup-tests.log`, `/tmp/musicos-startup-ui-tests.log`, `/tmp/musicos-startup-command.log`, `/tmp/musicos-startup-lazy.log`, `/tmp/musicos-startup-layout-recheck.log`, `/tmp/musicos-startup-package.log`.
