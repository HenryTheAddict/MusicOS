# Sparklist automatic setup and Your things — 2026-10-22

## Result

The desktop package includes the pinned, checksum-verified `clap-general-v1` model and its Apache-2.0 license. Enabled audio features automatically install it from the signed app resource directory. They no longer depend on an absolute build-checkout path or ask for a folder during normal setup. This is an offline bundle retrieval, not a new public model download service; the pinned manifest and experimental beta consent remain unchanged.

- Existing enabled profiles retrieve the model when the library is visible. Enabling Sparklists in Settings/creation also starts installation automatically.
- Shared requests deduplicate installation. Progress refreshes only while visible installation needs it; backend work survives navigation. Cancellation/errors require explicit retry; model removal switches the feature off so it cannot immediately reinstall.
- Installer validates exact sizes and SHA256, checks free space plus a 64 MiB margin, retains cancellation/atomic publication, and copies the license. Existing installed models/embeddings and saved sequences are preserved.
- Playlist suggestions refresh after model setup and analysis changes. Preparing songs is still an explicit action; installing a model does not start scanning/embedding the whole library.
- The suggestions action uses a padded, rounded themed button, fixed-size centered icons, and an arrow. It targets the Sparklist Audio subpage directly; generic audio-quality actions keep their original destination.
- Personal Home, its sidebar destination, the saved-playlist box, and Home's collection heading now read **Your things**. Library type tabs remain Albums, Artists, Songs, Playlists, and Sparklists.

## Build contract

`node scripts/prepare-spark-model.mjs` validates the pinned manifest, three model artifacts, and license before atomically staging the desktop resource bundle. Generated model files are ignored; only `.gitkeep` is tracked. Default source is the retained evaluated candidate under `out/rebuild-qa/model-evaluation-general/release-candidate/clap-general-v1`. Set `MUSICOS_SPARK_MODEL_SOURCE` when building in a clean or isolated checkout. Missing/unverified model inputs fail packaging instead of silently shipping a broken automatic-setup feature.

This change is desktop-only. Embedded audio and ONNX runtime bytes, OS source, database schemas, generation version, saved collection identities/sequences, and model approval status are unchanged. The app package grows by 281,234,043 model bytes plus its manifest/license.

## Verification

- 123 UI unit tests pass (39 files), including installation deduplication, automatic setup, completion invalidation, cancellation, retry, hidden polling, and removal behavior.
- 18 targeted Rust Sparklist tests pass, including packaged resource source resolution, checksum rejection, cancellation/retry cleanup, candidate consent, and audio ranking.
- 23 relevant browser checks passed across the main run and focused corrected runs: automatic installation for already-enabled/just-enabled profiles, exact Sparklist page routing, themed action geometry, retained playlists, names, Home/mix behavior, and the existing scale/theme matrices.
- The initial browser run had two selector failures caused by renamed/duplicate headings; accessible region names stayed descriptive, and the two corrected tests passed. An added unit test needed a complete bridge mock; TypeScript also rejected a test-only locator option. Both were corrected before the final build.
- Existing cached model state stays visible during refresh. Hidden Settings pages and completed installations do not gain idle polling.

Evidence is in the isolated worktree's `out/rebuild-qa/spark-auto-*`. Native delivery and exact build/model/runtime hashes are appended after signed installation. Listening quality, physical touchscreen behavior, Windows, and Linux remain separate acceptance gates.
