# Artwork, audio setup, icon, and documentation — 2026-10-12

Packaged Apple Silicon executable SHA-256: `b548174ab24631a26228a7290092ad03fd824326f98aafde59b60af9c6628054`.

Bundle: `target/release/bundle/macos/MusicOS.app`. Delivery copy: `out/rebuild-qa/polish-delivery/MusicOS.app`. Deep ad-hoc signature verification passed. This supersedes the build identity in [scroll/hover delivery](scroll-depth-hover-qa.md).

## Changes

- Removed album pointer-depth/tilt and jelly hover. Optional scrolling motion remains separate. Open-album artwork is still and clickable.
- Full-size album/artist image viewer uses the original image, themed surface, Escape/backdrop dismissal, keyboard focus, and focus restoration. Closing the viewer no longer reopens artist stats through programmatic focus.
- Elastic edge scrolling applies impulses through spring velocity rather than snapping its position for every wheel event. Audio opens with a finite soft overshoot, disabled under reduced motion.
- Audio settings are separated into Playback, Quality & analysis, and Sparklist. The quality control opens Quality & analysis; creation opens the Sparklist section. Hover/focus specifications remain available.
- Added Rescan audio and explicit full recheck, using the existing worker and priority/cancel/pause paths. Full recheck bypasses successful DSP results; valid old results survive unsuccessful refreshes. Specifications include duration. Source files and collection records are untouched.
- Sparklist creation/setup uses the supplied icon, shows insufficient analyzed coverage, and requires at least two analyzed songs in creation. Consent remains explicit and model validation remains pinned.
- Apple Vision landmark detection, larger detection input, conservative headroom, versioned negative cache invalidation, and content-based portrait keys. Corrupt portrait/depth cache images are rebuilt. Repeated unchanged model-readiness notifications no longer clear prepared depth caches.
- Packaged the existing blue `src-tauri/icons/appicon.png` into native icon resources. `CFBundleIconFile` points to `icon.icns`; its bundled SHA-256 matches the generated ICNS (`0f9bbace3c25550018448054c214d9cffa76afe149f60275c4979fa29581cb4d`). The Icon Composer source is preserved.
- Added a linked [repository user guide](guide/README.md) and [architecture documentation](desktop-architecture.md). No GitHub Pages configuration or deployment was added.

## Passed

- Production UI and packaged application builds.
- 47 frontend unit tests across 15 files.
- 85-test Tauri Rust suite, followed by the additional real-audio full-rescan regression: cached analysis reused in normal mode, replaced in full mode, source bytes and title preserved.
- Nine focused browser checks covering portrait focus dismissal, optional depth/time toggles at both window sizes and four interface sizes, quality settings navigation, full-size viewers, compositor alignment, reduced motion, still album hover, and elastic settlement. Six relevant checks rerun after the cache-notification fix.
- Four Audio/queue browser checks: both presentations, focus/persistence, duplicates, pointer/keyboard moves, large-queue autoscroll, Escape, and native file-import isolation in the browser harness.
- Actual local portrait: framed successfully with landmarks; second independent process reused the same cached crop in 90 ms. Original source retained. This is one real portrait plus orientation/group/uncertainty unit fixtures, not universal face-detection acceptance.
- Actual F32 depth map generated from the final portrait crop; another process resolved the same persistent map. Cold model checksum verification is still included on process startup; disk-cache reuse is not a claim that startup is instantaneous.
- Native Sparklist probe with eight real audio embeddings: verified candidate installation, no-consent rejection, deterministic offline generation, Choose For Me, feedback, save/refresh/conversion/reload, and model removal in an isolated directory. Muted same-album transition scheduling and pause/source-time probe also passed. Report: `out/rebuild-qa/polish-native-audio-beta-report.json`.
- Main app was stopped at Labor of Love, 2:04/2:04. Updated build launched without interrupting music; subsequent native inspection retained that stopped track/source time, 443 songs, six favorites, and three saved collections. Native quality shortcut opened the correct section with Rescan audio and full-recheck controls visible.

Embedded sidecar source SHA-256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime source remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

## Open gates

- No SHARP/“Insane depth” product toggle: current Apple model license excludes product development; a compatible licensed model/native renderer remains a research gate. No model assets published.
- Audible DJ listening, macOS remote media commands, and blind Sparklist listening acceptance remain open. The native probe is muted and is not packaged UI/listening acceptance.
- Current build's full native 120 Hz small/standard/wide and all-scale matrix was not rerun. Earlier measured roughly 120 Hz callback cadence remains documented separately; presented-frame/drop guarantees are not claimed. Private WebKit preference compatibility remains limited to the tested runtime.
- Queue and full-rescan packaged interaction matrix, broad portrait/provider cases, all adaptive palette combinations, and Windows/Linux remain incomplete/unverified where not explicitly tested above.

Local logs: `/tmp/musicos-polish-unit.log`, `/tmp/musicos-polish-rust.log`, `/tmp/musicos-polish-rescan-native.log`, `/tmp/musicos-polish-browser.log`, `/tmp/musicos-polish-browser-final.log`, `/tmp/musicos-polish-audio-menu.log`, `/tmp/musicos-polish-native-portrait.log`, `/tmp/musicos-polish-native-portrait-restart.log`, `/tmp/musicos-polish-depth-native.log`, `/tmp/musicos-polish-depth-restart.log`, `/tmp/musicos-polish-spark-native.log`, `/tmp/musicos-polish-delivery-build.log`. Temporary logs are not durable release artifacts.
