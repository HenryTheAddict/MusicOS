# Desktop consistency repair — 2026-10-21

The desktop repair is isolated in `codex/desktop-heart-preservation` at `0487cdf` and applied to the OS branch as `d35021f`. OS services, firmware work, saved collection sequences, installed models, and the database schema are unchanged.

## Delivered behavior

- Small favorites stay beside the playing title; active-song hearts use the playing palette and retain analyzed-tempo pulses. Transport and queue glyphs share explicit centers.
- The original reversible 360 ms Play/Pause contour morph is restored. Only playback and collection controls receive soft, theme-derived shading. Decorative SVG titles no longer expose filenames.
- Search and Command-K share a grouped music-first panel. An exact UTOPIA album match precedes its songs. The selected Sketch dropdown header remains stationary and connects to its choices.
- Appearance includes Album / Mini Player / Artwork cover-click behavior, defaulting to Album. Album-name navigation remains independent. Overflowing labels fade and reveal on hover/focus, respecting Reduced Motion.
- The mini-player wrapper is transparent, removing its rectangular backing behind the rounded surface. Visible controls retain their positions between artwork and queue views.
- Settings retains visited pages and drafts. Sparklist setup keeps cached status visible, deduplicates reads, and polls only visible running work. Installed-model, preparation, resume and diagnostic states retain existing safety checks.

## Verification

- Production UI build, 109 UI unit tests, all 140 native/Tauri tests, and the combined OS-branch desktop compile check passed.
- A 50-case browser pass covered player geometry at 720×480 and 1280×800 at 80/100/125/150%, theme ownership, metadata controls, Reduced Motion, retained queue rows/scroll, launch, touch simulation, and deep scrolling with depth off/on.
- A second 50-case pass covered Settings draft/scroll/secret handling, remembered mini-player views, pointer/keyboard resize requests, and the complete 40-case metadata matrix (both window sizes, four scales and five themes).
- Eight final favorite/alpha cases passed after the final styling correction. All four outer mini-player corner pixels have zero alpha at both 1× and 2× density.
- Historical checks expecting Settings to reset to Appearance and known artist photos to acquire a blur/loading overlay were updated to the accepted persistence and ordinary-image behavior.

## Signed build and live state

Application: `target/release/bundle/macos/MusicOS.app`. Signature verification passes; signing is local ad-hoc, without notarization.

| Component | SHA-256 |
| --- | --- |
| Desktop executable | `24dd4f084a647b6d5daa6d057ec9fc7494ad28d9c492718abe2af781baa7ffee` |
| Preserved embedded musicos-mac | `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d` |
| Preserved embedded ONNX | `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920` |

The previous app and the new package are retained in `releases/desktop-consistency-2026-10-21/`. A private checkpoint holds the current preferences/configuration and a verified backup of the configured library database.

MusicOS was closed for replacement and opened paused. Native inspection retained 507 songs, 20 favorites, saved mixes, and Borderline at 3:14. Unified search ranked UTOPIA first; the rounded mini-player and aligned transport were inspected at Retina density. Automated native interaction stopped when the user resumed using the main window; their newer appearance choices were not restored from the checkpoint.

## Remaining acceptance gates

The display reports 120 Hz, but current packaged animation frame intervals have not been captured. This is not a 120 fps claim. Complete the native AppKit pointer/keyboard resize and pin/context pass when it will not interfere with the user. Human motion/design acceptance, physical touchscreen behavior and Windows/Linux remain separate gates. Browser simulations do not establish these hardware results.

Retained evidence: `out/qa/os-foundation/desktop-consistency/` (verification summary, release record, workspace test report and screenshots).
