# File organization and loose singles — October 15, 2026

## Behavior

Settings → Library → Music folders → Smartly organize my files opens a review. Original files move only after explicit Apply, into Artist/Album beneath each current source. Album artist takes precedence. Filenames, encoded bytes and file tags remain unchanged. Managed/offline/symlink files are skipped. Existing targets are never replaced; collisions get a stable track-ID suffix. A stale library revision or changed checksum rejects a review before any moves begin.

Paths and an undo journal update transactionally before removing original links. If deletion fails, both verified links remain and Undo can restore the original DB path. A process interruption cannot leave the database pointing at a removed original: target paths are durable first. A crash before DB commit may leave an extra destination link; subsequent previews never overwrite it. Already completed moves stay undoable when a later file fails. The filesystem must support same-volume hard links. Companion files and empty folders are retained.

Applying requires paused/stopped playback. Queued paths refresh without recreating entry IDs, duplicates, pins or position; accepted DJ plans cancel for replanning. No source records or memberships are deleted. No original files were organized during development; file operation tests use temporary fixtures.

Singles use explicit release tags (case/spacing/MP4 freeform variants), verified release classifications, or explicit album-tag suffixes. Tagged singles lacking album tags use their song title for release grouping, with user album overrides protected. No track-count or folder-name classification is introduced. Cache v5 refreshes old negative tag results incrementally.

## Verification

- 113 app tests passed serially, including four filesystem organization tests and seven desktop metadata tests.
- 79 desktop-core tests passed, including duplicate queue entries/pins/current position retained after path refresh.
- 68 frontend tests passed; UI build passed.
- Browser compact (720×480) and wide (1280×800) preview, no implicit Apply, explicit Apply result, Escape/focus restoration and bounded long paths passed. Compact screenshot inspected separately.
- Real WAV tag round-trip in a temporary loose-files folder proves Single detection with no album tag or Single folder; MP4 freeform keys tested through the tag parser. No provider lookup was needed.
- Regression tests found and corrected a macOS `/var` → `/private/var` canonical-path mismatch in Undo.
- Human music listening, actual 120 Hz stress and Windows/Linux remain unverified; this pass does not claim audio quality changes.

Package identity/native review are recorded after signing below.

## Delivered build

- Ad-hoc signed Apple Silicon app, 98.38 MiB; deep/strict codesign verification passed.
- Executable SHA-256: `509c8f291a9d2e380d348af2edf57f389f7e7959e53892f5068fed24646fa78b`.
- Loaded while paused. Paused source position restored to 8.1 seconds. Native state retains 477 songs, 11 favorites and saved collections; user listening values .62×/−8.3 st linked, volume 100%, flat eight-band EQ and reverb off verified. Embedded sidecar and ONNX unchanged.
- Native read-only preview on the preceding candidate resolved 477 reviewed moves with full from/to paths; closed without Apply. Final candidate adds cache-upgrade retention only; UI and file operation behavior are unchanged. Tests verify old valid metadata remains visible while refreshing and changed bytes invalidate it.
- WKWebView suspended DOM updates while MusicOS was backgrounded during preview checks. Raising the app immediately showed the completed preview; process sampling showed idle workers, not a stuck hash worker.
- Logs: `/tmp/musicos-organize-full-tests.log`, `/tmp/musicos-organize-core.log`, `/tmp/musicos-organize-ui-tests.log`, `/tmp/musicos-organize-browser.log`, `/tmp/musicos-organize-package.log`.
