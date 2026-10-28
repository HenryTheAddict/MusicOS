# Player composition repair — 2026-10-28

The expanded player now follows the existing musicOS Figtree, supplied Sketch glyphs, small heart, colored surfaces and two-tone transport. The screenshot's nested full-window card and large gaps are removed. Identity, seek and transport share one bounded column; short wide windows use a side-by-side composition. The macOS overlay title bar has its own dragging space. Overflow moves to the header; favorite sits beside the title; shuffle and repeat flank transport; volume is a secondary thin rail. Opening, closing, artwork transfer, queue identities and acknowledged playback remain on the existing owners.

The Android player uses the same hierarchy, hides the redundant app header while expanded, and uses an artwork/identity row when height is limited. Dark text inherits the active ink at the application surface. No sync, service, playback protocol, database, or model changes.

## Evidence

- `npm run build:ui`: passed.
- `npm run test:ui`: 161 tests passed.
- WebKit: six adaptive/player tests passed, including retained root history, persistent player across resizing, queue identities, nested Escape, focus restoration, pointer Space, More geometry, empty playback, long text and Reduced Motion.
- Player composition matrix: 480×480, 480×800, 720×480 and 1280×800; 80/100/125/150%; Light/Dark blue. Asserted square art, 44 px controls, no horizontal overflow, native title-bar clearance, close/queue controls and no playback/mini-player commands from resizing or navigation.
- Existing adaptive-shell matrix also covered Light/Dark artwork, blue and extreme custom surfaces at 480/720/1280 widths, all four scales.
- Chromium: both player composition tests passed. Optional historical performance comparison skipped because no matching installed-source fixture server was provided; this is not a native FPS claim. Reduced Motion leaves no active player animation after completion.
- Android release build, local unit tests and lint passed. Android 17 emulator presentation test passed with Light/Dark, 360×720 and 360×480 content, actual fixture artwork, long titles, duplicate queue entries, repeated view changes, close and fully visible volume. The first screenshot pass found short-height volume clipping and missing explicit Dark title ink; both were corrected and screenshots re-inspected.
- Screenshot evidence under `out/mobile-player-audit-2026-10-27/`: `playing-*`, `queue-*`, and final Android images in `android-screenshots/qa/`. The folder name follows the original audit date; final checks ran October 28.

## Delivery

Built on `codex/desktop-heart-preservation`. Primary OS branch remains untouched. Native application signature verified; embedded sidecar/runtime/model hashes must match the installed app before replacement. Installation requires a fresh paused/stopped check, a private profile/library checkpoint, and preservation of the previous app. Signed hashes and post-load status are recorded in `releases/desktop-mobile-player-2026-10-28/release.json` in the primary workspace.

Physical Android touch, real phone font scaling/rotation, Bluetooth/headphone behavior and device-specific system chrome remain hardware acceptance gates. This change does not claim those were validated on a physical phone.

Installed and visually checked on October 28. Native Now playing and Up next were inspected in a narrowed window, then the player was closed and wide desktop presentation restored. Pain remained paused at 28.4 seconds. All three latest profile files and the saved-collection fingerprint match the private pre-install checkpoint; 508 tracks, 35 favorites, two manual playlists/five entries retained. All eight embedded sidecar/runtime/model hashes are unchanged; primary OS repository is clean.

Signed desktop executable SHA-256: `750261b127a06c62cbfc5c97350777d081c8f3371e888d6c730f4ecf116edecb`.

Signed Android APK SHA-256: `bc4708ff59fda4c933d61017e780e2302e6dbeb075efbcc1173d284c6d6a02ab`.

Desktop source commit: `f541247`. Archived delivery: `/Users/henry/Developer/MusicOS/releases/desktop-mobile-player-2026-10-28/`, including the previous app and revised APK. Android presentation was emulator verified; this APK has not been installed on a physical phone. The temporary test emulator was stopped after screenshots were inspected.
