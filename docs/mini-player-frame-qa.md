# Mini-player frame, themed menus, and progress — 2026-10-17

This supersedes the intermediate native-popup approach in player-feedback-qa.md.

## Final implementation

- Mini-player uses a single complete macOS borderless style mask, applied after queued Tao changes. Normal mode restores the complete titled overlay mask. It retains focusability and has a transparent native/webview background, so rounded CSS edges do not sit on an opaque rectangle.
- Window operations serialize through a dedicated asynchronous mutex. No ordinary state mutex stays held while waiting on the UI thread. The command acknowledges the AppKit callback before the frontend changes presentation.
- Dragging starts through Tauri's authorized window-drag API from text/empty space. Buttons and seeking are excluded. No added grab handle or whole-player invisible overlay.
- Mini-player actions are themed HTML controls. The native window temporarily provides menu space while the player itself stays 334×48 or 244×320. Closing restores the compact size/position before dispatching navigation or queue actions. Menu placement clamps to the monitor work area. Outside click and Escape dismiss; native focus reconciliation does not immediately dismiss it.
- The real Library/Settings/Devices trigger remains visible and stationary. Only other section choices extend below it. Font size, label position, and selected color do not change on opening.
- Progress uses a rounded rail and a width-based rounded fill, preserving end-cap shape at partial positions. Seek clamping, pending/error handling, and accepted DJ marks are retained.
- Now Playing, mini-player copy, and collection captions stay left aligned. Dark surfaces stay neutral; secondary artist text uses a readable theme color. Page/grid/elastic control entrances remain removed. Gentle depth and the initial launch color fade remain.

## Automated evidence

- 22 WebKit checks passed: fixed section-header geometry/text, compact touch, menu keyboard dismissal, menu theming, mini-player modes/direct queue controls, neutral dark contrast, deep/reverse scrolling with depth off/on, and queue operations.
- Final focus/icon correction: all eight player WebKit cases passed again.
- 132 native library tests passed. Active Tauri check and production TypeScript/Vite build passed.
- Final UI unit count and signed identity are recorded below. Early failures are retained in /tmp/musicos-themed-mini-*.log; an unrelated initial navigation/timer timeout occurred during concurrent QA. No listening or 120 Hz acceptance is claimed from browser fixtures.

## Native acceptance

Final packaged observations are appended below. Files, file tags, credentials, collection membership, and saved artwork are not edited by QA. The user changed playback/favorites during the session; the pre-load state was Pain, paused at 39.6 seconds, 507 songs, and 14 favorites. Windows/Linux remain unverified.

## Final packaged result

- Final executable SHA-256: `ba9f5d72a917252d3ab6acc7a498a9ff52bfa74591740a18061839b4c6aa58e2`.
- Bundle: `target/release/bundle/macos/MusicOS.app`, 99.15 MiB. Ad-hoc outer signature applied; strict deep verification passed.
- Preserved sidecar SHA-256: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- Preserved ONNX runtime SHA-256: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
- 79/79 UI units; 132/132 native tests; 22/22 WebKit player/section/scroll/queue checks. The final contrast-only correction passed all eight player WebKit checks again. Production build and git diff --check passed.
- Native checks confirmed borderless slim/artwork layouts, themed menu opening and Show artwork selection, direct Up next restoring the full window with nine upcoming entries, stationary section-header text, neutral Dark surfaces, readable artist labels, rounded progress caps, and readable album Shuffle artwork. A text-area drag gesture completed without an automation error; exact displacement was not measured.
- Refreshing the CUA app binding after native window-size changes was necessary: earlier attempts used stale geometry and mis-targeted clicks. Playback position varied during interactive QA, so exact seek preservation over the entire session is not claimed. The final reload retained Pain paused at 57.2 seconds; 507 songs, 14 favorites, and the existing collection list were present before and after it. The app is loaded and left paused in the album view.
- Remaining gates: physical 120 Hz frame pacing, measured drag displacement, cross-monitor/fullscreen restoration, and human listening acceptance. Windows/Linux remain unverified. No source/tag/credential edits or intentional favorite/membership changes were made by QA.
# Superseded queue presentation

The later Sketch queue mini-player adds Up next within the compact window. See [mini-queue-sketch-qa.md](mini-queue-sketch-qa.md) for the current design and package identity. The restoration and frame checks below describe the preceding build.

