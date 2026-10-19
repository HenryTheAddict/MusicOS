# Now Playing, Mini Player, and scroll repairs

2026-10-16. Current Sketch layouts and solid theme surfaces are retained.

## Changes

- Now Playing artist and album credits navigate to their existing library identities. The title, credits, and progress group are centered. Grid captions are centered independently of moving artwork.
- Now Playing right-click / Shift-F10 offers favorite, manual playlist membership, artist/album navigation, song details, and Mini Player. The dots beside Library now have a separate accessible action menu.
- Mini Player follows Sketch's Long Miniplayer (235444B6-9477-4BD9-B309-02EB961EB4D4, 334×48) and Taller Miniplayer (C1D799DC-B3A1-4474-9E1C-A3A7288A3B0D). The artwork layout is 244×320. Both use the existing playback state, queue, seek, and favorite commands. Native entry saves window bounds/fullscreen/maximized/topmost state, uses a compact always-on-top window, and restores the previous geometry. Escape returns to MusicOS. Artwork hover is restrained and respects Reduced Motion.
- The selected section is the open dropdown's header at the closed control's position. No duplicate selected row or header bounce. Pointer-release handling prevents the opening touch gesture from immediately closing it.
- Grid/virtualizer resize observations are coalesced by frame. Rounded-artwork measurements ignore root theme-token writes and skip unchanged clip paths. Dynamic Settings rows and scrolling containers use rounded CSS surfaces rather than dimension-dependent path masks. Settings entrances stay opaque.
- Touch queue gestures pan vertically rather than begin mouse reordering. Compact layouts keep the queue and Audio controls available with 44px touch targets.
- Depth yields to ordinary artwork during fast flings and resumes cached effects after scrolling settles. It keeps 3px hover / 1.5px gentle scroll bounds, RAM/texture budgets, and context-loss recovery. Ordinary artwork remains visible throughout.
- Retired src-desktop and its launcher: these were the unused SDL/C/C++ prototype, not the authoritative playback service. Active SVGs now live in assets/icons/ui; AssetIcon replaces the SketchIcon code name. Vector geometry and source provenance remain intact. Embedded runtime binaries are preserved byte-for-byte.

## Verification

- Native library tests: 132 passed. Workspace and production TypeScript checks recorded in /tmp/musicos-mini-*.log.
- UI unit tests: 74 passed.
- WebKit functional and layout checks include 1,000 albums, 10,000 tracks, deep reversals, compact touch controls, hover alignment, GPU/context recovery, Reduced Motion, dark contrast, Now Playing actions, and both player layouts. Metadata layouts were checked at 720/1280 widths, four interface scales, blue/artwork/custom palettes.
- Early fast-wheel stress runs intermittently stalled after a reverse wheel at a virtual-row gap. This is recorded as a failure, not hidden by retries. Final stress results and packaged checks are appended below.
- No user music files or collection membership are edited by QA. Browser fixtures isolate favorites and playlist changes.

## Acceptance limits

Packaged native bounds restoration, actual trackpad edge bounce, audible playback continuity, and frame intervals on an actual 120Hz display require native inspection. Automated WebKit fixtures do not constitute those checks. macOS UI inspection initially failed because the Mac was locked. Windows and Linux are unverified.

Runtime SHA-256: 6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d.
ONNX runtime SHA-256: ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920.

Additional diagnosis: the reverse-fling trace recorded late virtualizer scrollTo writes while newly visible uniform album rows were being measured. Grid rows now use their known layout height and disable measurement-driven scroll compensation. This retains native input and removes those stale offset writes. It does not introduce a wheel handler.

The broader workspace check found 16 compile errors in the retained legacy GPUI Control Center (undefined cx, AlbumId, and artwork_for_track). They are unrelated to SVG path resolution and remain a separate legacy target limitation. The active Tauri target and its 132 library tests pass. The historical C++ QA document is an archive; that standalone prototype has now been retired.

## Final automated results

- Fixed-height grid regression: 42/42 WebKit tests passed across three repetitions of dropdown/deep-scroll/touch and Now Playing/Mini Player cases.
- Final source: 16/16 WebKit and 16/16 Chromium checks passed, including depth context recovery and Reduced Motion. The wheel regression also asserts at most one initial scrollTo call, so measurement corrections cannot silently fight the wheel.
- The previous metadata matrix passed all 29 relevant cases (two widths × four scales × three palettes plus direct editing, album review, manager navigation, block reordering, and sidebar drops). Earlier failures and traces remain under out/rebuild-qa and /tmp/musicos-*.log.
- Native library 132/132; UI units 74/74; production TypeScript/Vite build passed. No listening acceptance or 120Hz performance claim is made.
- Mini Player restoration explicitly restores the macOS overlay titlebar as well as geometry. Native restoration still requires an unlocked Mac.

## Packaged identity and handoff

- Bundle: target/release/bundle/macos/MusicOS.app (99.13 MiB).
- Ad-hoc outer-bundle signature applied; codesign --verify --deep --strict passed.
- Packaged executable SHA-256: `6992c772b4716dddcdc69ffed431d9257f38282815c6798fff8ce22a3baefd1b`.
- Packaged sidecar and ONNX hashes exactly match the preserved hashes above.
- Delivery checklist: source/build checks complete; signature/hash checks complete; load pending; native Mini Player restoration pending; actual trackpad/120Hz frame pacing pending; audible playback continuity pending.
- The final attempt to inspect playback was blocked by the macOS lock screen. The agent did not quit or relaunch the playing app. Loading must wait until the Mac is unlocked and playback is confirmed paused/stopped, using the user's existing authorization to pause and load.

## Native follow-up — 2026-10-17

The Mac was unlocked and the update was loaded only after the transport showed Play (paused). Native checks confirmed individual contributor navigation (James Blake), album navigation (UTOPIA), Now Playing context actions, the manual-only playlist picker, and both mini-player layouts. The library retained 507 songs, 13 favorites, and the same collection list. QA did not add/remove playlist members or favorites. Dark mode was visually inspected and the original Light mode restored.

Native mini-player exit exposed competing macOS borderless/resizable style-mask updates. The final implementation keeps the overlay mask intact, hides/shows standard window buttons, and restores the titlebar container hierarchy. Late header alignment commands are ignored while Mini Player is active. The final build showed enabled Window → Zoom and visible close/minimize/zoom controls. Zooming wide produced a correctly aligned seven-column grid. Returning from native Zoom produced stale/clipped captures; foreground verification is pending because native coordinate activation returned noWindowsAvailable. This is not marked as a resize pass.

Native song scrolling reached entry 507, immediately reversed to the 460s, and returned to entry 1. This was native input, not manual seeking. Playback stayed paused on TIL FURTHER NOTICE. The first attempted seek restoration to 304.8 seconds eventually displayed the song end, 314 seconds; exact seek-position preservation is therefore not claimed.

Final signed executable SHA-256 superseding the prior package: `18682afcd506681533dc31a82b05b0cc108cf014853d3259ae83615eebeb5da9`.
Strict deep signature verification passed. Packaged sidecar and ONNX hashes are unchanged. This build is loaded.

Observed native diagnostics at 2026-10-16 23:58:26 EDT: 169 frame intervals, median 8 ms, p95 10 ms, CPU draw p95 6 ms, 3/169 intervals above 12.5 ms (1.8%). The diagnostic stored interface_percent=100 during startup and does not establish the active display mode or exact dropped-frame count. These are observed samples from the earlier native check, not a 120 fps guarantee or final resize acceptance. Actual 120Hz display verification and listening acceptance remain open.

Final native follow-up: after returning from Zoom, a later capture showed the restored five-column grid with matching square artwork masks, centered captions, and aligned toolbar. The earlier capture was transient/stale; resize return is now visually confirmed after settling, but foreground live-resize timing remains unmeasured. Final native library tests passed 132/132 in 9.79 s, and git diff --check passed. Playback remains paused.
