# Reliable everyday library — verification, 2026-10-16

This pass implements coordinated database writes, album-first metadata review, responsive manual editing, optional journaled file-tag updates, direct manual-playlist editing and local suggested additions. It also updates release classification, removed-source cleanup, artwork renditions, solid appearance controls, compact Settings and quiet lazy-loading placeholders.

The latest depth direction is **gentle hover and scrolling**: hover displacement is capped at 3 px, scroll displacement at 1.5 px with a quick return to neutral. Native momentum and bounce remain; there is no whole-page stretch, blur, box tilt or scroll zoom. Prepared depth resources retain the existing 256 MiB RAM / 128 MiB texture budgets. Ordinary artwork remains the fallback during preparation and GPU recovery.

Discord cover lookup now uses album-level artist identity, conservative title normalization and bounded retries for transient misses. Local/manual images cannot be exposed directly to Discord; catalog artwork availability and Discord rendering remain external dependencies.

## Automated checks

| Check | Result |
| --- | --- |
| `cargo test -p musicos-init --lib` | 43 passed |
| `cargo test -p musicos-tauri --lib` | 132 passed in the final full run |
| Desktop enrichment focused tests | 7 passed |
| Final organization recovery tests | 5 passed |
| File-tag tests (included in native suite) | 4 passed |
| UI unit tests | 74 passed |
| Everyday browser checks before final regressions | 27 passed |
| Gentle hover/scroll depth checks | 2 passed |
| Final browser regressions | 27 passed; Ignore/Restore passed on corrected rerun; grouped drag passed three consecutive reruns |

The database stress fixture runs interactive metadata edits concurrently with cache writes, canonical artwork preparation and file organization. Tests cover stale reviews, rollback, canceled moves, recovery links, source forgetting, membership preservation, suggestion deduplication, appearance migration and Discord retry identity.

File tests use disposable fixtures. FLAC, ALAC and MP3 tag round-trips preserve decoded audio; Undo restores byte-exact originals. MusicOS-only edits leave file bytes unchanged. Later-edit conflicts, interrupted publication, backup removal and failed staging are checked. No personal music files are retagged, moved or forgotten during QA.

Browser coverage includes 720×480 and 1280×800 at 80/100/125/150 percent in blue, artwork and custom Dark configurations. Screenshots are inspected separately from geometry assertions. Direct row selection, Delete, immediate Add Music, details-sheet saves, album artwork confirmation and the Sketch dropdown have dedicated checks. Queue regression checks cover repeated entries, insertion, dragging, Escape and focus; quick Audio stays independent.

Depth checks cover bounded displacement, scroll alignment, return to neutral, cache reuse after context loss, texture budget and Reduced Motion fallback. These tests do not substitute for measured physical display presentation.

## Delivery and acceptance limits

Build identity and packaged smoke results are recorded below after signing and loading. The embedded playback sidecar and ONNX runtime are preserved.

- Automated output measurements do not constitute human listening acceptance for reverb, pitch/time processing or DJ transitions.
- No claims are made about improved model recommendation quality; the pinned experimental weights and consent remain unchanged.
- The built-in Liquid Retina XDR display is available, but its active refresh rate is not reported by `system_profiler`. Active rAF intervals, when recorded, are callback cadence rather than OS-presented frames or definitive dropped-frame counts. Actual 120 Hz presentation acceptance remains separate.
- Live Discord image display requires a connected Discord client and a matching remotely accessible catalog cover. IPC payload/retry tests do not prove the remote client displayed it.
- Provider matching, unusual file formats, permissions on external volumes and crash/power-loss behavior beyond the journal fixtures require further real-world acceptance.
- Windows and Linux remain unverified.

Radio and Hidden Gems remain deferred. Original files, canonical artwork, source/track identities, saved collections and existing playback queues are preserved except for explicit user-reviewed operations.

## Failures found during delivery checks

- Initial packaging failed TypeScript validation because the existing native organization recovery action was missing from the typed UI command union. The contract now includes recovery.
- The first packaged smoke surfaced `SQLITE_CANTOPEN` during an artwork refresh burst. A verified high-resolution upgrade had updated a song reference while leaving the canonical album path on the old image. Every generic library refresh invalidated artwork and requested the upgrade again. The correction updates canonical references in the same transaction, coalesces library refreshes, skips database access for ordinary renditions, and serializes legacy upgrades before opening SQLite. Regression tests verify canonical reference publication and the no-database ordinary path. Final native restart verification is recorded below.
- Ignore/Restore browser QA initially reloaded before the action finished, then matched two same-title fixture tracks. It now waits for completion and checks the reviewed candidate only; the persistence/restoration case passes.
- One grouped-drag browser run targeted a row scrolled above the viewport. Scrolling the target into view before calculating drag coordinates yields three consecutive passes. The initial failure is retained in `/tmp/musicos-refresh-final.log`.

The final navigation/depth rerun passed seven other checks; the grouped-drag case was rerun separately as described above. No broad-suite pass is claimed for legacy tests that still expect the intentionally removed checkbox editor.

## Native rendering observation and backend profile

On the packaged Apple Silicon app at 125% scale, recent active depth samples in a 1023×671 viewport contained 75–138 intervals: median rAF 17 ms, p95 18 ms, p95 CPU draw time 1 ms. All sampled intervals exceeded the diagnostic 12.5 ms threshold used to flag missed 120 Hz budgets. This is approximately 60 Hz callback cadence, **not a verified 120 fps result** and not a measurement of physical dropped frames. Active display refresh remains unknown. These samples occurred during normal native artwork navigation; they are not an isolated GPU benchmark.

A separate unoptimized backend profiler used a temporary SQLite copy of the 507-song / 52-album library. First-process uncached artwork validation took 6371.04 ms; a fresh process reusing validation took 11.43 ms. Median warmed complete sampled backend phases were 380.90 ms and 326.14 ms respectively. Debug compilation and concurrent QA materially affect these numbers; they are not comparable with the previous release-mode profile and do not measure process launch to first interactive paint.

The refresh-loop repair was verified in the native app: launch no longer showed a database-open error, collections loaded, and SQLite file handles settled at 9 rather than 135 in the failing run. Configured managed-source protection is now also reflected in the UI, not only rejected by the backend.

## Final signed delivery

- Bundle: `/Users/henry/Developer/MusicOS/target/release/bundle/macos/MusicOS.app` (99.12 MiB).
- Executable SHA-256: `91b128185bebccc257d870dfbf68e1418d6b8306165340b5888afb14ab8209be`.
- Local ad-hoc signature; `codesign --verify --deep --strict` passed. This is not a notarized distribution build.
- Embedded sidecar is byte-identical: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- Embedded ONNX runtime is byte-identical: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
- Loaded only after pausing playback. The initial transport showed zero after restart; the observed paused Stateside position was explicitly restored to 111.4 seconds and verified in the native transport. The saved queue and current song remained available.
- Native library shows 507 songs, 11 favorites, one manual playlist and nine audio Sparklists. Saved sequences were not refreshed or edited during checks.
- Personal collections heading opens the shared manager. Final Settings exposes appearance controls, album-first Suggestions, Albums/Songs manual selection and Album Artist editing. The configured MusicOS Media source has no Forget action; empty legacy entries retain it. No real metadata changes, tag writes, file moves or record forgetting were applied for QA.
- Native screenshot inspection confirms the new placeholder color follows the adaptive theme. Three final browser checks passed for flat Settings plus 1280×800 blue/100% and 720×480 custom Dark/150% metadata layouts. `git diff --check` passes.

Logs: `/tmp/musicos-everyday-package.log`, `/tmp/musicos-everyday-native-final.log`, `/tmp/musicos-everyday-ui-tests.log`, `/tmp/musicos-everyday-browser.log`, `/tmp/musicos-everyday-regression.log`, `/tmp/musicos-ignored-final.log`, `/tmp/musicos-drag-final.log`, `/tmp/musicos-refresh-final.log`, `/tmp/musicos-contrast-final.log`, `/tmp/musicos-artwork-upgrade-final.log`, `/tmp/musicos-everyday-backend-profile.json`.

Reference behavior: [Apple playlist editing](https://support.apple.com/en-ie/guide/music/musd5d051981/mac), [MusicBrainz API](https://musicbrainz.org/doc/MusicBrainz_API). These references do not imply use of Apple's private playback or recommendation engines.
