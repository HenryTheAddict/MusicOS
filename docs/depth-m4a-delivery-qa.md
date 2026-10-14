# Depth, portraits, M4A and DJ delivery — 2026-10-12

Historical build record below. The later [scroll and album-hover delivery](scroll-depth-hover-qa.md) supersedes its build identity and 60 Hz rendering result; outstanding audio/listening acceptance gates remain open.

Packaged Apple Silicon executable SHA-256: `02dd236a566a944c44c50387e2f194561b523b71f34fbfa117b50827981ebb84`.
Bundle: `target/release/bundle/macos/MusicOS.app`; deep ad-hoc signature verification passed. Delivery copy: `out/rebuild-qa/polish-delivery/MusicOS.app`.

## Passed

- Exact Apple DepthAnythingV2SmallF32 package pinned to revision `cfef6f6f2a70783dedc0bfae40cecbc2052285d3`; all three checksums verified; native Core ML inference produced valid maps. Installed model is 99,188,950 bytes. Depth preference remains opt-in. F32 improves precision, not parameter count.
- Stronger scroll response, bounded depth warping with edge feathering, fixed viewport grid compositor, artist detail pointer movement, stale-cover cleanup, hover neutralization, overscroll artwork blur, reduced-motion restoration.
- Browser verification: 46 unit tests; 5 focused interaction/WebGL tests; 4 renderer/layout checks including 32 scale/palette/size captures. These are not native refresh-rate acceptance.
- 82 Tauri Rust tests, plus final focused artist alias regression; 53 desktop Rust tests.
- All 33 actual library M4As decoded and sought successfully; AAC and E-AC-3 plus generated ALAC fixture. Shared decoder serves playback, DJ decks, analysis and audio embeddings.
- Muted native DJ probes: supported same-album overlap, late preparation fallback, AAC-to-E-AC-3 deck preparation, mixer acknowledgement, pause/seek cancellation and source-time checks. Not audible listening acceptance.
- Packaged F32 QA app: scroll alignment and artist navigation inspected with real maps; foreground rendering sample median 17 ms, p95 18 ms, CPU draw p95 <=1 ms at 100% interface scale, 1280x800 logical window.
- Main app observed paused at Midnight City 17.1 seconds, six favorites and three saved collections. Latest process PID 86571 started 16:15:41 local; subsequent UI inspection retained paused source time and collections. User navigation was detected during inspection; no further restart attempted.
- Embedded sidecar SHA-256 unchanged: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- ONNX runtime SHA-256 unchanged: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

## Open acceptance gates

- 120 Hz display target FAILED: measured compositor cadence approximately 60 Hz, not 120 fps; full native small/standard/wide scale matrix remains incomplete.
- Audible DJ transition listening, clipping/pitch/source-time listening acceptance, macOS system remote command exercise, and Sparklist blind listening evaluation remain unverified. Sparklist remains experimental; no model release assets published.
- Windows and Linux remain unverified.

Logs: `/tmp/musicos-f32-final-delivery.log`, `/tmp/musicos-f32-rust.log`, `/tmp/musicos-f32-unit-final.log`, `/tmp/musicos-f32-browser-final.log`, `/tmp/musicos-f32-renderer-final.log`, `/tmp/musicos-depth-model-f32-qa.log`, `/tmp/musicos-m4a-qa.log`, `/tmp/musicos-m4a-dj-qa.log`. Temporary logs are local, not durable release artifacts.
