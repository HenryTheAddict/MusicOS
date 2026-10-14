# Desktop architecture and verification

[User guide](guide/README.md)

## Boundaries

React renders the desktop interface; typed bridge commands call Tauri/Rust services. The embedded sidecar and ONNX runtime remain separate preserved resources. Local library records, collections, metadata overrides, queue-entry identities, and analysis caches have independent lifecycles.

The macOS decoded-audio interface supplies playback, both DJ decks, DSP analysis, and Sparklist preprocessing. Decoding, seeking, inference, and spectral work remain off the audio callback. Transition marks reflect playback-acknowledged schedules.

## Artwork path

Canonical album resolution feeds all album-related images and adaptive colors. Apple Vision produces optional framed portrait copies without rewriting originals. Content hashes plus processing/model versions identify persistent portrait/depth results. Depth uses a serialized inference worker, atomic map publication, disk validation, and bounded prepared/GPU caches. Visible grids share a compositor; ordinary artwork remains available as fallback. Album hover has no pointer-depth or jelly transform.

The current high-refresh workaround uses guarded private WKPreferences selectors scoped to the app's WebView. Missing selectors and exceptions preserve platform defaults. This is not a public API compatibility guarantee; do not represent it as App Store acceptance.

## Audio rescan

Changed/outdated mode selects missing or invalid DSP/metadata results. Full mode explicitly bypasses current DSP results. Both probe specifications in the existing background worker, preserve old successful cache rows until successful replacement, retain manual playback priority, and support pause/cancel checkpoints. Neither mode rewrites source tags or collection membership. Embeddings retain their independent content/model-version invalidation.

## Acceptance

Use production UI builds, frontend interaction tests, targeted Rust cache/metadata tests, then a signed packaged macOS app with isolated data. Compare real screens separately from geometry assertions. Record build SHA-256, display refresh, callback cadence percentiles, slow intervals, scale, window size, and sample count. Callback cadence is not equivalent to presentation timing or exact dropped-frame counts.

Test rescan failures, cancellation, persistence, changed files, user overrides, duplicate queue entries, source-time accounting, reduced motion, rapid reversals, artwork identity, offline cache reuse, and checksum rejection. Audible DJ tests, remote media commands, and blind model listening need explicit acceptance beyond browser or muted tests.

Latest record: [depth hover and quick Audio](depth-quick-audio-qa.md). Previous record: [grouped Settings and Sparklist ranking](grouped-settings-qa.md). Previous record: [artwork/audio polish](artwork-audio-polish-qa.md). Historical records: [depth/M4A](depth-m4a-delivery-qa.md), [scroll/hover](scroll-depth-hover-qa.md). Consult the newest delivery record for the current build; old hashes and results remain historical evidence.

Latest collection/metadata and artwork verification: [delivery checklist](collections-metadata-delivery-qa.md).
