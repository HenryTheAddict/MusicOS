# Collections, metadata, artwork, and Sparklist delivery

Tested on 2026-10-13 on Apple Silicon macOS.

## Package and loading

- Bundle: `target/release/bundle/macos/MusicOS.app`.
- Executable SHA-256: `8971441eeb8bbac427278b3db9a68b4532b82ebd35021872e406516cf71be53e`.
- Production packaging and strict deep ad-hoc signature verification passed. This is not a Developer ID/notarization claim.
- The final native process started after this executable was built/signed. The new collection manager was visible in the packaged app.
- Native session retained 443 tracks, nine favorites, one manual playlist and five audio Sparklists. No collection was saved, deleted, refreshed, or converted during native UI verification.
- No Quit, Pause, or transport action was issued during loading. The user began navigating/creating a playlist during verification; the interaction guard was respected and further UI actions stopped. Music continued.
- Embedded sidecar remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; embedded ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

## Changes

One Library manager now separates Playlists, audio Sparklists, and Smart playlists, with search, recent/name sort, existing artwork, counts, duration and contextual creation. Settings links to it instead of hosting redirect-only collection sections. The obsolete always-unavailable Sparklist page and lossy single-rule editor were removed. Smart editing preserves every rule, all/any matching, and limits.

Settings uses flat grouped rows. Sparklist setup distinguishes consent, installation, preparation, readiness and errors, with model/storage/diagnostics behind Details. Consent and pinned-model validation remain explicit. Model removal is available for an installed model, including when consent is off. Setup and embedding coverage do not imply recommendation or beat/key accuracy.

Metadata now follows Find → Review → Apply. Background indexing prepares suggestions; high-confidence matches no longer automatically apply fields or canonical artwork. Selected-field review includes current/suggested values, release/source links, artwork previews and stale-revision/candidate checks. Manual editing retains selection through filtering, distinguishes multiple values, supports unchanged/set/clear choices and requires a bulk summary before application.

Additive database history records before/after values, protection flags and canonical artwork references in the edit transaction. Undo checks for conflicts and restores whole batches atomically. History persists and supports sequential Undo; the UI displays the latest 50 batches. Artist profile cache status/retry is separate from song editing.

The user's IGOR final song had track number zero and a `12 ` filename prefix. Missing numbers can now recover this filename form for display/playback order without rewriting tags or explicit user-number overrides. Unknown numbers sort last. All track records remain intact.

Expanded depth waits for its opening animation before geometry measurement, retains the source artwork version, and clears prepared memory caches on explicit artwork invalidation. Prepared full/cropped textures use the map's content identity; texture preparation is serialized and corrupt cache textures are rejected. Source framing, full image aspect ratio, SDR presentation and ordinary-image fallback remain intact.

New generated artwork uses version two for additional soft fields, blobs, ribbons and rounded-star arrangements; saved version-one covers keep their renderer. Preview offers sound, mood and seed-cover color sources. Editable names include mood labels, color-family labels and cleaned song-title forms such as November-esque. Typed names survive back/forward navigation. Artwork changes do not rerank or regenerate songs.

## Verification

| Check | Result |
| --- | --- |
| Tauri/native contracts | 96 passed |
| Desktop services, including review-only high-confidence matching and explicit retry of kept suggestions | 57 passed |
| Core library/import tests | 39 passed |
| Frontend tests, including rule preservation, field selection, setup states, artwork versions and names | 60 passed |
| Focused collection, Settings, Audio, viewer, setup and sidebar browser tests | 13 passed |
| Depth renderer tests, including final overlay alignment, pointer movement, cache reveal and GPU fallback | 6 passed |
| Corrected management palette matrix | 40 combinations passed: 720×480 and 1280×800, four scales, four palette overrides and actual adaptive playback colors |
| Existing bulk-edit and playlist/rule regression journeys | 2 passed |
| Native collection manager and restored user session | Passed |
| Native persistent depth-cache reuse for actual IGOR artwork | Passed; existing map, `fresh=false`, unchanged modification time, second lookup 23 ms |
| Final native expanded-view/onboarding/metadata UI walkthrough | Partially verified; user interaction stopped further navigation |

The native depth probe's first cached lookup took 3,284 ms in a debug executable, including initial model verification. This was a cache hit, not fresh inference; the warm lookup was 23 ms. Neither figure establishes release rendering frame rate.

Screenshots: `out/rebuild-qa/collection-manager.png`, `management-*`, and `expanded-depth-aligned.png`. Logs: `/tmp/musicos-management-{unit,native,core,browser,depth-browser,palette-matrix,collection-bulk,depth-native,package-final}.log` and `/tmp/musicos-manager-desktop.log`.

## Remaining acceptance

Audible DJmode quality, macOS remote-command behavior, blind Sparklist listening, representative annotated analysis accuracy, and a full presented-frame measurement on a 120 Hz display remain separate open gates. No model weights were replaced or published. Windows/Linux remain unverified. Lyrics and stems remain deferred.

User documentation: [Collections](guide/collections.md), [Metadata](guide/metadata.md), [Sparklists](guide/sparklists.md), [Artwork](guide/artwork.md).
