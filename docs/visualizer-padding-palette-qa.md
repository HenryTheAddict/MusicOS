# Visualizer transport spacing and album palette updates — 2026-11-03

The expanded player's fixed 64 px Play/Pause target previously contained a rem-sized glyph. At 125% it measured 65 px, overflowed its padded target, and was off-center. Previous/Next had the same issue. The visualizer dock also reserved only 104 px for 112 px of content and padding.

The shared dock now centers 52 px Play/Pause geometry inside a 64 px target, 32 px skip glyphs inside 44 px targets, and 20 px secondary glyphs inside 44 px targets. All have zero button padding; the visualizer dock reserves its full 116 px, including equal 8 px vertical insets. Supplied SVG paths and the 360 ms reversible morph remain intact.

Album colors are keyed by artwork source when a cover is known, retaining ready colors on same-album skips and synchronously restoring already-read artwork. Unknown embedded artwork remains keyed by track. Late results cannot replace the current source. Explicit artwork invalidation refreshes the current palette and rejects the pre-invalidation response. No playback-clock update restarts extraction.

The supplied Visualizer master C21A68F4-A394-48E0-B8CA-EF1F1B41BD4C and Settings master B09B450E-551F-4401-91B6-641A7BCEBEF9 were exported from Sketch document C8FFC865-CF40-43FB-AA71-0B45DBA1EF0C. Original 32×32 geometry/strokes are retained; only the white export-canvas use is removed. IDs/checksums are recorded in assets/icons/ui/source.json. Visualizer now identifies its context-menu action and preset picker; Settings identifies visualizer options. Both were inspected through Sketch screenshots and in the native silent fixture.

Validation:

- UI build and 224 tests across 71 files pass.
- Browser checks cover 480×480, 720×480, and 1280×800; 80/100/125/150%; Light, Dark, blue, and extreme custom themes; all three views. Glyphs stay centered within 0.1 px and inset at least 6 px. Dock controls remain inside their surface. These 16 layout/lifecycle checks pass.
- The additional morph check passes: Play/Pause target and glyph bounds stay identical through rapid reversal and completion. Total: 17 browser checks.
- Palette regressions cover raw swatches, delayed responses, same-album skips, revisiting artwork, replacement/invalidation, and track-specific embedded artwork.
- Native silent Metal fixture confirms the supplied icons and dock's visual alignment. Browser tests establish the interaction/morph bounds; no user playback was changed during fixture verification.
- This delivery includes the preceding uninstalled reading/focus correction documented in visualizer-reading-focus-qa.md, including hue-aware GPU recoloring and suppression of the oversized focus rectangle.

Live Automix listening, every Mini Player entry/exit path, Apple authentication, and physical trackpad behavior are not newly accepted by this focused repair. Installation requires a fresh native paused/stopped check. Preserve the previous app, latest profile, collections, and all eight protected runtime/model files.
