# Sparklist length and measured DJ cues — 2026-10-14

## Delivered behavior

Sparklist creation has a compact, keyboard-accessible 10–100 song slider, default 25, beneath the vibe controls. The number occupies a fixed-width area. Changing it clears the preview without triggering generation or clearing the seed, mood, feedback, or manual artwork. Preview and Choose for me carry the requested length; previews report actual count and duration, with a shortfall notice when needed. Long totals use hours/minutes. Saved mix editing changes the next explicit refresh's target only. Saved sequences and active queues remain independent.

`Generation.target_count` defaults to 25 when absent from legacy JSON. Generation, preview validation and storage validate 10–100 and use the configured cap. Collection responses expose optional `target_count`; a `length` collection action atomically updates only the saved generation setting. No database migration, model replacement or consent change is needed.

DJ candidates use actual outgoing/incoming beat timestamps, measured overlap durations and local tempo ratios. Every aligned beat must remain within 50 ms predicted error. Structural boundary confidence is distinct from beat confidence; at least three recurring cycles must support inferred boundary phase. Phrase scoring considers confidence, drift, local density/energy, harmony and skipped material. Both outgoing overlap boundaries and incoming entry respect confident repeated sections. Local grids inconsistent with the constant-ratio mixer are rejected. Octave agreement uses measured beats backed by onset analysis rather than forcing an arbitrary half/double metadata ratio.

The coordinator requests cues at least two wall-clock seconds ahead, expressed against the current source-speed baseline. Playback still owns acceptance, late-cue fallback, decoder deadlines, cancellation and promotion. Accepted schedules remain protected. Diagnostics include score components, predicted drift and rejected tempo/instability/drift counts. Full song, ±6% tempo limits, manual pins, safe ending blends and accepted-only rail marks remain intact.

DSP cache version is now `dsp-5-measured-phrases-11025-4096-512`, with `structure-4-measured-phrases-11025-2048-256`. Upcoming stale results refresh incrementally; older same-file results remain readable until a successful replacement. A failed refresh test verifies they survive. Changed source identities still invalidate cached results.

## Automated and native results

| Check | Result |
| --- | --- |
| Rust app tests | 105 passed |
| Desktop playback/core tests | 75 passed |
| Frontend unit tests | 64 passed |
| Length browser matrix | Passed compact/wide windows, all four scales, blue/light/dark/vivid token sets, keyboard bounds and stable value width |
| Saved length browser path | Passed; songs unchanged after editing length |
| Combined collection/length browser regressions | Final run: 10 passed. An earlier model-gate assertion had an ambiguous status locator because output is also a status element; it now scopes to model setup. |
| Offline real cached embeddings | 443 recordings; exact 10/25/100 lengths, deterministic ranking, save/reopen and length-edit sequence preservation; disposable saved collections only |
| All cached recording pairs | 195,806 eligible pairs: previous planner 42 supported plans, measured planner 53. Maximum predicted drift 278.64 ms before, 48.16 ms after. Cached estimates are not ground truth. |
| Fresh structural analysis | 16 real recordings, 240 pairs, 46 supported plans; maximum predicted drift 44.12 ms; read-only sources, no user cache writes |
| Native measured transition | Moon Crystal → Dyscalculia, accepted → active → promoted, no seeking, muted native output, about 100.4 s cue; disposable queue |
| Native ending fallback | Two automatic overlaps/promotions across three real short recordings, no seeking, muted output, disposable queue |
| App preview | Packaged app generated 100 Dreamy songs using real cached embeddings; no preview playback or saving; existing paused position restored |

Reports are under `out/rebuild-qa/spark-length-dj/`. The read-only pair probe and isolated native/offline probes are reproducible examples in `src-tauri/examples/`.

## Acceptance limits

A public independently annotated four-beat clip was exercised: the tracker deliberately returned no confident grid (F1 0 at 70 ms, safe fallback). It is insufficient for the tracker's minimum evidence requirements and cannot establish full-song accuracy. Broader independently annotated section/beat benchmarks and human listening remain open. Real cached-pair drift is a consistency measurement against estimated beats, not independently verified musical accuracy.

The native output probes were muted and do not establish subjective transition quality, audible clipping/pitch quality, or model listening acceptance. Sparklist remains experimental. No model assets were published. Windows/Linux and 120 Hz frame pacing remain unverified. The existing embedded sidecar and ONNX runtime are unchanged. Source files, collections, artwork and queue identities were preserved.

## Research references

Apple documents audio-aware transitions using beat matching and time stretching in its [AutoMix overview](https://www.apple.com/newsroom/2025/06/apple-services-deliver-powerful-features-and-intelligent-updates-to-users-this-fall/). Algoriddim documents [automatic entry/exit regions, blend duration and tempo adjustment](https://help.algoriddim.com/user-manual/djay-pro-mac/mixing-basics/using-automix), and describes [Fluid Beatgrid following tempo fluctuations](https://www.algoriddim.com/press_releases/447-algoriddim-unveils-djay-pro-5-with-next-generation-neural-mix-crossfader-fusion-and-fluid-beatgrid-). These describe reference behavior. MusicOS's measured-boundary scoring is its own implementation; neither proprietary engine nor stem separation was integrated.

## Packaged identity

Executable SHA-256: `68e223f97cd5f7ec22923c9b0ad98f17c2bc4a4470a0ea3fe251467781307825`.

Deep ad-hoc signing and strict signature verification passed. Embedded sidecar SHA-256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

An initial packaged revision was loaded while paused and generated a real 100-song Dreamy preview without playback or saving. Its long total exposed the old minutes-only format, corrected to hours/minutes in the final build. The final build also includes the protected-section endpoint regression test. macOS initially locked before final reload. On 2026-10-15, after unlock, the final signed build was loaded while paused. Packaged keyboard adjustment to 100 and a real Dreamy preview passed; the summary displayed **100 songs · 6 hr 37 min**. The unsaved verification preview was dismissed without playback. DON'T TRY remained paused at 205.6 seconds; 443 songs, 10 favorites and existing collections were restored.
