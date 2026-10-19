# Artwork completion stability and theme cleanup — 2026-10-17

The previous fix removed one opacity animation, but several other handoffs remained. Artist cards covered existing photos with a blurred loading layer during source/framing updates. Routine depth layout notifications hid the whole compositor. Depth preparation could paint a different-resolution original over an idle ordinary image, and background artist-photo completion could replace the visible page's palette.

Changes:

- Image replacements decode before URL, original identity, and depth revision are published together. The prior image stays visible during a delayed replacement. Obsolete decode completions are ignored; changing collection identity or explicitly clearing artwork does not carry an old image into a different collection.
- Cached artist photos do not receive the blurred loading overlay. Missing-photo progress remains available.
- Depth preparation alone does not paint an idle photo. Routine layout updates keep the current compositor visible until its next measured frame. Grid/portrait textures use the displayed, decoded rendition, with its identity in the prepared-image cache key. Full viewers retain original-resolution preparation.
- Shared prepared caches are retained during invalidation; displayed identity/revision changes invalidate the affected resources. Genuine context loss still falls back to ordinary artwork and recovers from cached data.
- The current collection's palette remains stable through background image/profile updates; navigating to a different collection resolves its palette. Explicit artwork invalidation can refresh it. Original artwork palette records and appearance preferences are not modified.
- Neutral artwork receives a blue accent instead of a muddy gray-green accent. Light artwork themes use restrained, solid tints. Dark surfaces remain neutral charcoal; section controls and selected navigation use quieter surfaces with readable accent cues.

## Automated checks

UI build passed. 86 UI unit tests passed, including delayed/failed/obsolete image decoding, identity changes, explicit clearing, embedded-artwork recovery, portrait framing, theme contrast, and monochrome accent handling.

Browser suites passed for Albums/Artists idle preparation and layout notifications, fresh-map completion without opacity effects, cache/context recovery, gentle hover/scrolling, large-list scrolling and touch reversal, player controls, Dark mode, and launch fade. Native inspection and build identity follow after delivery. No source files, queue entries, model weights, or audio processing are changed by this update.

Live provider latency, human perception during a long cold analysis run, 120 Hz timing, and Windows/Linux remain separate acceptance gates; static screenshots alone do not establish absence of every transient frame.

## Package and native loading

Final ad-hoc signed macOS executable SHA-256: `ca23ce1e060b2942f5c9148259478843d2c03a71f13c9c7ded03d5eaac5f4557`. Deep strict signature verification passed. Embedded playback and ONNX runtime hashes remain unchanged (`6b1fa66d…`, `ff52828c…`).

86 unit tests passed. The final targeted browser runs passed all four hover/depth checks, nine player checks, four launch/theme checks, and ten deep-scroll/touch checks; three panel/control checks also passed during this repair. Expected Reduced Motion color assertions were updated to the intentional new solid tint. The no-artwork startup path was corrected and retested.

Loaded after the user unlocked the Mac. Playback was confirmed paused before quitting the preceding build and relaunching the signed package; the executable hash was rechecked. Native Albums and Artists screenshots showed readable left-aligned captions, ordinary artwork and cached artist portraits without loading overlays, and consistent solid theme surfaces. Both grids scrolled to the bottom and back to the top successfully; Albums was restored afterward. These observations verify the displayed states and scroll reversal, not every intermediate frame or a fresh long analysis run.

Stateside remained paused at 34.1 seconds, with 507 songs, 14 favorites, and the user's Illegal-esque collection present after restart. These checks did not edit collections, favorites, queue entries, or appearance preferences. Dark-mode contrast and delayed analysis handoffs were covered by automated tests; this native pass used the user's current light artwork theme.
