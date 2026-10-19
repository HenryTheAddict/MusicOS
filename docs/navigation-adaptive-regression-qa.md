# Navigation, animation, and adaptive-color regression repair

2026-10-18, macOS. Follow-up to the mixed performance results in `persistent-settings-metadata-reverb-qa.md`.

## Reproduced failures

- A native Fancy That album view retained blue controls/surfaces despite its red artwork and saved Artwork color source. Its canonical song cover is a 4000×4000 PNG, 29,076,484 bytes. Palette extraction rejected originals above 20 MiB and silently fell back. A second threshold treated low-brightness red as monochrome because it measured absolute RGB range instead of relative saturation.
- The frontend light palette whitened already-curated artwork surfaces again. A separate last-view cache excluded the artwork path and could retain a failed/obsolete result when the cover changed.
- A broad `animation:none!important` rule suppressed ordinary context menus. Navigation animated only captions, leaving most transitions visually absent.
- Library unmounted when opening Settings. Its image nodes and depth compositor/textures were rebuilt on return. A shared lazy-loading Suspense boundary could also tear down Library layout effects during first Settings entry, even after retaining the Library DOM.

## Repairs

- Sample palettes through the pixel-bounded, cached 128-pixel rendition path. Valid originals up to 128 MiB and 36 megapixels are supported. Smaller renditions reuse an available larger cached rendition, avoiding another full-resolution decode. Originals are preserved; thumbnails never upscale into larger views. Existing valid canonical palette caches remain valid.
- Light Artwork mode uses the curated artwork background/surface directly at the default tint, with saturation/tint adjustments and contrast checks. Dark surfaces remain neutral charcoal with artwork-colored accents. Dark red is recognized as colored; near-gray artwork keeps a clean fallback accent.
- Resolve themes from actual source paths and track identities through the existing deduplicated palette service. Removed the redundant stale last-view cache. Late responses cannot overwrite a newer view; changing preferences does not restart the launch fade.
- Added 180 ms opacity-only content fades on navigation and Settings destination changes. Data, resize, scrolling, and analysis do not replay them. Reversals start from current opacity. Restored 140 ms menu motion. Reduced Motion disables these animations. No gradients, jelly stretching, or completion flashes were added.
- Retain Library DOM separately from Settings, with independent lazy-loading boundaries. Hidden grids ignore zero-width measurements. Depth suspends drawing/loading and releases visible-item pins while hidden, retaining resident resources within the existing shared budgets; returning reconciles geometry and resumes. Genuine context loss still recovers through prepared images. Background visibility no longer freezes the entire top-level HTML snapshot.

## Evidence

- Full UI unit run: 93 passed, followed by the final six appearance tests including the added dark-red case; 94 distinct UI tests covered in total.
- Provider palette tests: 9 passed, including an original above the old 20 MiB gate, deterministic cached output, color-area selection, and contrast.
- Rendition tests: 3 passed, preserving framing, no upscaling, and canonical-reference publication behavior.
- First targeted interaction run: 14/15 passed. The new retention test correctly found the remaining shared Suspense teardown. After separating boundaries, all three new regression tests passed.
- Broader interaction/dark-layout run: 19 passed, including rapid/deep scrolling, retained Settings/drafts/scroll, hidden polling/secrets, mini controls, dark artwork edges at 1×/2×, and all eight dark metadata size/scale cases.
- Depth stress run: 8 passed, including scrolling alignment, hover, Reduced Motion reversal, viewer focus, wide resize/reflow, rapid reverse scrolling, cache reuse, and GPU recovery.
- The retention regression confirms the same Library image and canvas objects after Settings navigation, no hidden drawing, and **zero additional texture uploads on return**. This is a verified reduction in repeated work, not a claim of measured native launch or frame-rate improvement.
- A read-only production sampler accepts the real 29 MB Fancy That cover and returns the same palette on repeat: accent `#210908`, background `#bfa8a7`, surface `#a07f7e`. Initial uncached debug sampling took approximately 5,665 ms; the cached lookup took 0.486 ms. That cold result motivated reuse of larger cached renditions. On a separate disposable copy, grid preparation took 6,603 ms in the debug helper; subsequent first palette sampling reused that cached grid rendition in 199 ms, then repeated in 0.353 ms. Those are component timings, not end-to-end app speed. Reports are retained in `out/persistent-qa/fancy-palette-grid-reuse.json`.

Logs and screenshots are under `out/persistent-qa/regression-*` and `adaptive-repaired.png`. Native confirmation: the old package showed blue surfaces/controls for Fancy That. The repaired package showed artwork-derived warm red surfaces and deep red controls with the same saved Light / Artwork / 150% saturation / 20% tint preferences. A Settings round-trip returned to the same Fancy That view. Returned to Albums, with The Recipe still paused at 324.3 seconds, 507 songs, and 17 favorites. Source files, collections, queue, and reverb implementation were unchanged. CUA capture intermittently reported user/window changes; fresh reads verified the completed actions.

Signed package executable SHA-256: `39f9d0b1d3b5a603f084e94ad87c9b8e58c7c3ab8929555326d930e18c1be66e`. All 12 current UI JS chunks were verified embedded. Strict deep ad-hoc signature verification passed; this is not Developer ID notarization. Embedded playback runtime remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`. Earlier launch medians/p95 are not reused as evidence that this repair makes launch faster. Human motion/feel acceptance, physical touchscreen behavior, native 120 Hz frame intervals, Windows, and Linux remain separate gates.
