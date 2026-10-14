# Depth hover, quick Audio and Sparklist delivery

## Implemented

- Cached pointer depth on album cards, album headers and expanded originals; no box rotation. Full-image map support preserves nonsquare viewer framing. Missing image-version handling fixed for the viewer.
- A blurred, darkened static artwork backdrop, readable caption and close control. SDR handoff retains the ordinary image. Fresh-map status and identity gate a single 220 ms reveal; cached remounts and polling do not replay it.
- Waveform opens quick Audio; queue icon opens Up next independently. Volume, three/eight-band EQ, pitch, speed, linking and Reset are native controls. Settings buttons are flat and queue presentation copy is updated.
- Global listening preferences use additive defaults and atomic persistence. Native workers compose user speed/pitch with separate DJ correction; mixer EQ uses prepared coefficients, bounded state, headroom, limiting and gain crossfades. Source reloading crossfades a bounded 20 ms history. Pause freezes source clocks. Source-time overlap and real-time overlap are separate fields so rail geometry remains correct at changed speed.
- DJ fallback retains an ending blend after preparation failure; routine next-track preparation cannot overwrite an existing plan. User-speed bounds choose a safe ending blend when a musical overlap would exceed bounds. Key compatibility and energy continuity contribute to candidate scoring; confidence gates remain unchanged.
- Sparklist creation is Choose a vibe → Preview → Name & save. Inline consent/install setup preserves seed selection. Model weights, checksum gate and experimental status are unchanged.
- Audio-derived abstract cover parameters follow inspected Sketch color fields, rounded blobs and stars. Versioned choices persist with saved collections, with explicit cover regeneration and manual alternatives. Playlist glyphs derive their tint from cover artwork; Sparklist controls get multicolor treatment without particles.

## Analysis candidate rejected

A full-track, read-only candidate scan covered 42 real recordings from Discovery, Junk and Currents, taking 479.948 seconds in a debug build. Both baseline and multiband candidate passed the existing beat-confidence gate for 36 songs, but the candidate reduced supported musical plans from 20 to 2 of 1,722 ordered pairs. It was **not promoted**.

Production retains `dsp-4-dp-onsets-11025-4096-512` and `structure-3-dp-onsets-11025-2048-256`. Read-only inspection confirmed all 443 local DSP cache records remained on the baseline version. Candidate functions are accessible only to benchmarking; they do not become default analysis. Final baseline planner coverage again measured 20 supported plans.

`analysis-benchmark` accepts independent beat annotations and reports one-to-one beat F1 at 70 ms tolerance. A pinned [madmom short recording and manual annotations](https://github.com/CPJKU/madmom/blob/27f032e8947204902c675e5e341a3faf5dc86dae/tests/data/annotations/sample.beats) were exercised: both analyzers returned unknown beats/tempo and fallback (F1 0). This verifies conservative handling of that short material; it establishes no general beat accuracy. A representative full-length annotated corpus and listening acceptance remain necessary before replacing the baseline analyzer.

## Verification

- 90 Tauri core tests, 56 desktop-native tests and 49 frontend tests passed.
- Ten focused browser tests passed for quick controls, linked/unlinked state, EQ preference persistence, quality navigation, both Up next presentations, queue dragging/duplicates/autoscroll, and image expansion at 720×480 / 1280×800.
- Five depth renderer checks passed, including hover, scroll alignment, one-time completion, expanded viewer depth, reduced motion and context-loss fallback.
- Settings/artist-strip screen matrix passed both window sizes, all four scales and four palette overrides. The entire new-feature native wide/scale/palette matrix was not repeated.
- Muted native real-file probe passed control acknowledgement, three/eight-band state, independent pitch/speed, pause and seek. A 1.5× setting measured 1.476 source seconds per real second over the probe window; polling cadence contributes measurement error. This is not audible pitch acceptance.
- Native Sparklist probe used eight actual audio embeddings: 30.236 seconds of inference, 281,234,043 model bytes, local generation, explicit feedback, Choose For Me, save/refresh/reopen and manual conversion. Generated cover persistence passed. Same-album transitions reached acknowledged schedule, activation and promotion; paused source time remained frozen. Volume was zero.
- Packaged native UI exposed actual FLAC specifications, volume, DJmode, Bass/Mid/Treble, speed, pitch, link and Reset while the user session was paused. No audible user-session test was performed. A subsequent user interaction guard was respected.
- Recent packaged renderer telemetry at scale 125%, 1,023×671 content area: 240 intervals, median 8 ms, p95 11 ms, draw CPU p95 4 ms; 4/240 intervals exceeded 12.5 ms. These intervals are a missed-refresh proxy, not compositor presentation counts. Hardware reports a built-in Liquid Retina XDR display but does not expose an exact refresh value through the inspected system report. Universal 120 fps acceptance is not claimed.

## Remaining acceptance

Audible musical quality, extreme user pitch/speed quality, clipping listening, macOS remote-command behavior and blind Sparklist listening remain open. Broader independently annotated beat/phrase evaluation and full native rendering matrix remain open. Windows/Linux remain unverified. No SHARP product integration or published model assets were added. Lyrics and stems remain deferred.

Native listening and real-recording reports are under `out/rebuild-qa/`; detailed execution logs are `/tmp/musicos-*`. Public test audio is a local QA artifact, not a bundled app asset.

## Final package identity and loading

Final Apple Silicon executable SHA-256: `0d0c91761bfa92be5b2f14c269fe2bb26109bb0786ddcd5d6cbeb7ac1d3fee09`.
Bundle: `target/release/bundle/macos/MusicOS.app`. Production build and strict deep ad-hoc signature verification passed.

The final package was loaded after the user unlocked the Mac, with playback verified paused before quitting. It restored 2SEATER at 3:12 / 6:49, 443 tracks, six favorites and four saved collections. Native UI verification confirmed the waveform opens quick Audio with actual FLAC specifications, volume, three-band EQ, speed, pitch and linking; the queue icon independently opens Up next with 24 upcoming entries. Listening preferences were not changed and music was not interrupted. The final package separates source-time transition overlap from real-time duration and completes specifications/presentation copy.

Embedded sidecar remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; embedded ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
