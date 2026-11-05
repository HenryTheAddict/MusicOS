# Fluid navigation and setup repair — 2026-11-04

## Change

Album and artist cover transfers keep the outgoing pixels visible through route commit, measure the destination after scroll restoration, and retarget an interrupted layout from the displayed image. The cover owns the 320 ms movement; page and album-strip effects no longer move its anchor. Late album descriptions cannot stretch the square artwork. Scroll anchoring is disabled on the page scroller, and resolved artist information/expanded biography state is retained on return.

Onboarding is now two steps, Appearance and Music folders, with a live inert library preview, optional name, filled choices, explicit native folder selection, and reachable footer. Existing preferences and first-run completion behavior are retained. Sparklist setup uses a gradient preview, a compact status/action layout, and expandable details; model installation and analysis commands are unchanged.

## Verification

- Full frontend unit suite: **242 passed** (75 files).
- Browser navigation/setup suite: **13 passed**. Covers cover intermediate geometry, rapid reversal, asynchronous description layout, scroll restoration, artist album-strip return, retargeting, expanded biography, Reduced Motion, onboarding persistence, and automatic Sparklist setup.
- Onboarding inspected at 480×480, 720×480, 1280×800, four interface scales, Light/Dark blue; responsive controls stay within the dialog. Theme colors continue to use the existing palette resolver.
- Frontend production build and native macOS app build passed. Native build deliberately reused the existing embedded runtime/models.
- Ad-hoc signed bundle passes deep/strict signature verification. All eight protected files match the previous delivery.
- Installed-app inspection and final profile comparison are recorded with the release manifest after loading.

This delivery does not claim new physical trackpad, controller, audible Automix, or Linux hardware acceptance. The existing Linux AppImage is a separate artifact and is not rebuilt for this macOS UI repair.

## Installed delivery

- Source: `0133889505d4a48a40e5bdaa676bd9565d1502ec`.
- Signed executable SHA-256: `80336033d1f5bdf8f4d07cf33dbaef194bb90addde6fe7f6a4275016b98c0f2b`.
- Installed after fresh native Play-button verification (paused), with previous app and latest SQLite/preferences checkpoint retained. The restored track was Outro Freestyle/4ever at 0:01. Playback subsequently resumed through user interaction and was left running.
- Installed native album and artist surfaces were inspected. Native setup inspection was not completed during concurrent user interaction; setup validation uses browser screenshots and the automated size/scale matrix.
- Library retained 625 tracks, 55 favorites, 2 manual playlists, and 6 manual playlist entries. Manual collection fingerprint and Sparklist sidecar are unchanged.
- All eight protected runtime/model hashes and the primary OS worktree are unchanged.
- Release bundle, previous app, profile checkpoint pointer, screenshots, logs, and manifest: `/Users/henry/Developer/MusicOS/releases/desktop-fluid-navigation-setup-2026-11-04`.
