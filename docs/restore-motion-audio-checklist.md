# Song motion and audio completion checklist

Implementation in progress, 2026-10-10. This document is an acceptance checklist,
not a declaration of release completion. Preserve embedded runtime and music.

## Implemented; verification in progress

- [x] Restore song-row Play reveal and 180 ms title/credit slide within its column.
- [x] Restore transport Play/Pause morph separately.
- [x] Appearance time toggles default on, atomically persisted, m:ss rail output.
- [x] Disc headers and local track numbers in the virtualized album sequence.
- [x] Suppress redundant album artist subtitles; retain mixed-list credits.
- [x] Themed Settings controls, scrollbars, Audio active state and artist facts.
- [x] Top-layer pointer/focus artist names with collision handling and dismissal.
- [x] Quality hover motion, unique popover identities and Audio now-playing control.
- [x] Remove automatic zero-duration transitions based on album adjacency.
- [x] Revalidate missed cues into future unwarped ending blends; retry failed preparation.
- [x] Only deck-ready scheduled/active transition regions appear on playback rail.
- [x] Bounded mixed-output telemetry; three-band meter outside audio callback.
- [x] Public macOS MediaPlayer commands, metadata, artwork and source position adapter.
- [x] Explicit Sparklist beta consent defaults off; only exact pinned general candidate accepted.
- [x] Checksum-verified staged model installation, progress/cancel/retry/removal.
- [x] Rust ONNX runtime included in desktop resources, with existing attribution.
- [x] Existing real cached-embedding generation/preview/feedback/save/refresh routes connected.

## Required acceptance work remaining

- [ ] Complete browser layout/interaction matrix at both sizes and all four scales.
- [ ] Verify palettes: blue, light, dark, vivid and adaptive, including contrast.
- [ ] Verify packaged macOS MediaPlayer commands and artwork in system UI.
- [ ] Verify actual automatic DJ transitions with real recordings, pause/seek/queue changes.
- [ ] Establish trustworthy continuous-passage detection. Album adjacency alone is insufficient;
      unknown passages currently receive normal confidence-aware blends.
- [ ] Validate spectral telemetry, limiting and pitch preservation in packaged playback.
- [ ] Install staged beta through packaged UI; verify offline analysis/generation and playback concurrency.
- [ ] Finish blind listening and annotated-recording analysis benchmarks before beta promotion.
- [x] Stage GitHub release assets in a draft for review; no public model download is enabled.
- [ ] Record final tested build SHA and load it only when music can remain uninterrupted.

Windows/Linux are unverified. Lyrics and stems are deferred. No production-model
approval or public publication is implied by the opt-in beta.

## Latest regression fixes, 2026-10-10 evening

- [x] Compact 2.75 rem song rows; playing track retains its number while output levels use the row Play slot.
- [x] Playback time sits beside the rail inside the original header height; corner rounding transitions both ways.
- [x] Collection rows use balanced trailing Shuffle spacing.
- [x] Song right-click Play next / Add to Up next; queue right-click Play now / Play next / Remove.
- [x] Pointer song dragging into Audio appends an independent queue entry; Escape cancels without native import.
- [x] Automatic recording enrichment preserves tagged album editions. Existing erroneous automatic album overrides are archived and removed; artist credits and user edits remain.
- [x] Explicitly accepted metadata review changes are protected as user edits.

Verification: 35 frontend unit tests; 4 desktop metadata tests; 4 enrichment tests.
The latest layout/interaction run passed 6 tests, including both viewport sizes and all 4 scales.
Existing Audio reorder tests passed (duplicate entries, keyboard movement, edge autoscroll and Escape).

Native isolated report: `out/rebuild-qa/native-audio-beta-report.json` verifies 8 actual local audio embeddings using the packaged ONNX runtime, deterministic offline generation, explicit feedback, sample-clocked same-album transition scheduling/activation, and pause source-time freeze. Test output was muted; audible listening and packaged UI acceptance remain pending.

Packaged regression build loaded while paused: SHA256 `ee4a66d7200b00540b7444690492f6a05a6f1c8f7ded04ccbc6c831de01d47be`. Native inspection confirms the corrected CHROMAKOPIA and M83 grouping, visible playing-row number, compact rows and unclipped header time. The user resumed listening; further restart/UI acceptance was deferred to avoid interruption. The repair archived 25 differing automatic album values; track records and the Doechii artist credit remain intact.

Follow-up spacing/motion revision: tightened playback-bar inset/gap and row vertical padding; moved the three audio-derived levels from the number into the row Play control. Supplied Play SVG morphs into levels over 180 ms and back on pause; current-row click toggles playback without restarting the song. Current-row title stays in place across pause/resume. Reduced motion switches immediately and shows static levels. Verification: 35 UI unit tests and 6 browser layout/interaction tests passed, covering both window sizes and all four interface scales.

Follow-up packaged build: `22378db30e3d2516f6bb934035d6d2ef3291cfcb02bb0d61415b8d1349ef118d`. Loaded into the intended macOS app while stopped; native screenshot verified compact header and album rows. Restart retained the stopped track and 6:21 source position. No audible acceptance or model promotion is implied.


## Consistency/artwork/Sparklist follow-up

See `docs/consistency-artwork-spark-qa.md` for the implemented changes and honest acceptance status. Packaged SHA256 `d2e422bd9a4f54d9796ddf9d0d8c7270ee3518759a021045c56f08f35f113280`; signature verified. Final loading and native UI inspection are pending because the Mac locked. The existing running app was left untouched. Real cached-embedding generation, saving/refresh/conversion/restart and muted sample-clocked transitions passed in an isolated native probe. Audible listening, system-control acceptance, and user visual approval remain pending.


2026-10-11: signed follow-up build `2ecef4a615b9fea1631cafe1381772cb3f364ad663c6b2265921e3cca48f5fd2` is now loaded. Native pointer song insertion into floating Audio passed; temporary entry removed and original queue/source position restored. Appearance time switches, adaptive artist controls, artwork and Sparklist consent state inspected. See `docs/consistency-artwork-spark-qa.md` for remaining acceptance gates.
