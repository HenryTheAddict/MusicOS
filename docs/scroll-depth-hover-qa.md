# Scroll and album-hover delivery — 2026-10-12

Historical record. Superseded by [artwork/audio polish](artwork-audio-polish-qa.md), which restores still album hover.

Packaged Apple Silicon executable SHA-256: `94a59f491d4bd9f16b8bc95b7ed1093fc8c89cbbbbada465a718085e193ca601`.

Bundle: `target/release/bundle/macos/MusicOS.app`. Delivery copy: `out/rebuild-qa/polish-delivery/MusicOS.app`. Deep ad-hoc signature verification passed.

## Delivered

- Pointer-responsive artwork on the large cover inside an open album, preserving header composition and About placement. No corner Play overlay. Reduced motion remains static.
- Bounded elastic edge scrolling with synchronized depth layers; quieter flat parallax without scroll-dependent zoom pumping.
- Single-pass, smoothed depth displacement bounded to 6 px to avoid fractured lettering.
- Original photo bytes and color profiles retained for the depth texture path; standard dynamic range on artwork layers. Persistent depth crops and bounded decoded-image/response caches reduce repeat work and layer-switch brightness differences.
- Guarded, app-local WebKit high-refresh preference. This uses private WKPreferences selectors because the public API does not expose this preference. Missing selectors or exceptions retain platform defaults; no global Safari settings changed. Compatibility beyond this tested macOS runtime is unverified.

## Verification

- Production UI build and `cargo check -p musicos-tauri` passed.
- 46 frontend unit tests across 14 files passed.
- Four focused browser depth tests passed: scroll/overscroll/navigation alignment, artist pointer/reduced motion, grid pointer/elastic settlement, and open-album pointer effect without overlay or layout changes.
- Rust texture-cache regression passed: original source bytes retained and cached depth crop reused.
- Final packaged app inspected in an isolated data directory with real library artwork, depth enabled and 125% interface scale. Discovery's open-album cover retained its layout and About section.
- Native 120 Hz display samples: 240-frame album/portrait samples had median requestAnimationFrame spacing 8 ms and p95 9–11 ms. Grid samples had median 8 ms and p95 11–12 ms. CPU draw p95 was 0–1 ms at timer resolution. These measure callback cadence, not guaranteed presented frames or an exact dropped-frame count. Full small/standard/wide and all-scale native matrix remains incomplete.
- Metrics saved in `out/rebuild-qa/scroll-depth-acceptance/final-native-metrics.json`. Source field `over_120hz_budget` counts intervals above 12.5 ms; it is not the fraction exceeding 8.33 ms.

Logs: `/tmp/musicos-album-hover-package.log`, `/tmp/musicos-album-hover-tests.log`, `/tmp/musicos-album-hover-unit.log`, `/tmp/musicos-scroll-native-check.log`, `/tmp/musicos-scroll-cache-test.log`. Temporary logs are local evidence, not durable release artifacts.

## Delivery and remaining acceptance

Main app was playing when delivery finished. Restart approval was requested; the main process was left running. The separate QA app uses copied data and has no active playback. Depth preferences and beta consent were not reset.

Embedded sidecar SHA-256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

Audible DJ transitions, macOS remote commands, and Sparklist blind listening acceptance remain open as recorded in the earlier delivery checklist. No model assets published. Windows/Linux unverified.
