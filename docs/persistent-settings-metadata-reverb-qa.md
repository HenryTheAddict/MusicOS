# Persistent Settings, metadata workspace, mini-player, and reverb QA

Date: 2026-10-18. macOS only. This records implementation and verification separately from design/listening acceptance.

## Changes

- Removed the horizontal mini-player. Artwork and queue share a resizable footprint, anchored bottom controls, and a saved view preference; legacy horizontal requests map to artwork. Queue DOM and scroll position survive toggles. Removed child movement/scaling in favor of 180 ms opacity changes. Menus/control states use 140 ms ease-out; Reduced Motion disables spatial motion.
- Removed an older queue rule that positioned its menu button independently. Regression assertions compare all six bottom button rectangles across views, not just their containers.
- Settings mounts once after its first visit. Categories and expensive subpages retain their DOM, category, scroll, search, and non-secret drafts. Hidden scopes are inert, stop foreground polling/drawing, and freeze child updates. Credential fields clear on hiding; file-operation confirmations/preflights are not retained as authorization.
- Added deduplicated, bounded display-data requests with in-flight invalidation handling and refresh in place. Metadata list/history/backups use display caching; mutation/preflight/credential requests do not. Library-change events carry additive reasons and optional affected IDs. Incoming metadata changes mark open reviews outdated without replacing drafts.
- Lightweight preferences/playback/task bootstrap and library loading run concurrently. Optional metadata/provider/integration work waits until after the library becomes usable. Required schema and durable file-operation recovery remain on startup. Idle imports warm Settings/collections.
- Metadata uses Review, Edit, and History destinations, with Ignored as a Review filter. Album browser and inspector sit beside each other at 48 rem of workspace width; smaller workspaces use detail navigation. Shared fields/artwork precede track comparisons. Unchanged fields collapse; provenance remains available. Manual rows use unchanged/set/clear, preserving Album Artist and contributors. Review/Apply counts and actions remain in a footer. Existing edition approval, stale tokens, Undo, backups, and default MusicOS-only application remain.
- Artwork/depth now share the DOM Sketch curve rather than layered mismatched clips. GPU masks follow CSS size and pixel density, with ordinary images visible throughout preparation/recovery. Prepared image and mask RAM together remain within 256 MiB; texture residency remains budgeted at 128 MiB. Dark surfaces omit pale backing/inset highlights.

## Reverb: failed listening gate and subsequent repair

The first packaged candidate had matching requested/applied controls and real wet processing, but the user explicitly reported **Still sounds dry**. That candidate failed listening acceptance. A nonzero processed-frame counter is not an audible result.

The prior processor fed four independent damped comb loops. It produced measurable wet output but relatively sparse, short late reflections. The replacement scatters delayed feedback through a normalized Hadamard matrix, with all lines read before writes. This orthogonal matrix preserves feedback energy; damping and feedback bounded below one make the loop lossy. Default decay is longer, excitation remains high-passed and energy compensated, and the wet return retains subtle stereo spread. Amount/depth remain smoothed and buffers preallocated. Dry remains unity; off is exact bypass before the existing safety limiter. Saved amounts and default-off behavior are unchanged. Startup also suppresses temporary Favorites counts and progress time until the lightweight/library bootstrap completes.

Technical reference: [Rocchesso and Smith, Circulant and Elliptic Feedback Delay Networks for Artificial Reverberation](https://www.di.univr.it/documenti/ArticoloRivista/allegato/allegato574545.pdf). This informs feedback structure; it is not evidence of human listening acceptance.

Production mixer captures use the same decoded material for dry/45%/100% output, eight seconds of music plus two seconds of tail. Source files were read only.

| Material / depth | Wet-minus-dry RMS at 45% | Tail energy | Music RMS change |
| --- | ---: | ---: | ---: |
| Human After All / 50% | 0.081917 | 264.425 | +0.080 dB |
| Glimmer / 100% | 0.063626 | 235.299 | +0.391 dB |
| One Beer / 0% | 0.041031 | 27.153 | +0.209 dB |

For identical Human After All material, the previous candidate measured 0.039302 wet difference RMS and 35.920 tail energy. Default impulse tail occupancy above 0.00001 between 0.1–1 seconds increased from 55.3% to 96.8%. Sustained 0.75 input peaks at 0.881 before limiting at default depth; impulse/rapid controls/invalid values/decay tests pass. Deep late energy decays to approximately 4.66e-12 in the final half-second of an eight-second impulse test.

Max-wet music can still engage the existing soft limiter: Human After All pre-limit peak 1.655, 13,094 samples above full scale among 960,000 samples including the tail. This is explicitly not a claim of zero limiter activity or a claim that the sound has been accepted. Captured final output is finite and bounded; the network passes stability checks before limiting.

Evidence: `out/persistent-qa/reverb-density-summary.json`, `reverb-density-stability.json`, and `reverb-density-{default,deep,onebeer}/`. The first candidate native main mixer reported Controls 1 / Applied 1, Room 45%, Depth 50%, with processed frames increasing from 0 to 69,745 on real playback. Final native verification is recorded below.

## Automated verification

- UI unit suite: 92 passing tests in 30 files.
- Desktop core after the final reverb change: 83 passing tests, including dense late reflections, stable rapid controls, exact dry bypass, pause, seek, parameter acknowledgements, automatic queue recovery, and existing output checks.
- Tauri suite: 133 passing tests, including additive preference migration. These were run before the DSP-only follow-up and CSS rule removal; neither changes Tauri command/storage logic.
- Metadata layout matrix: 40 passing cases, 720×480 and 1280×800, four scales, five palettes. Representative screenshots were inspected separately from overflow assertions.
- Final targeted interaction/depth suite: 15 passing cases, including Settings retention, hidden polling/secret clearing, mini-player view/footprint retention, heart pulses, depth reversals/recovery, and dark corners. The subsequent individual-button mini-player position regression also passes.
- Four corner pixel comparisons at 1×/2×: 40 samples outside the curve per case, maximum RGB difference 0 between ordinary artwork and active/recovered depth. This checks leaked backing pixels at those sampled coordinates; intentional bright image pixels are not classified as leaks.

Initial QA failures and repairs are retained in logs: corner assertions originally counted one corner rather than all four; a queue height assertion reflected the removed layout; one compact run was interrupted by development hot reload. Their targeted reruns passed. A test invoked without the Playwright config failed navigation setup; the configured rerun passed.

## Performance and native delivery

Seven fresh-process launches and twenty warm Appearance visits were collected on candidate `0ffa9715685a6b758fc6e24042f793a3a2bf3af32001976b6b14113782ff543e` (the final build subsequently only suppresses transient loading counts). They were collected on this Mac with the isolated 507-song catalog and silent playback. The after profile was cloned separately to protect the user's QA edits. The catalog was restored to the baseline fields through Undo on a disposable clone; revision/history values therefore are not byte-identical. Filesystem/provider caches were warm, and other apps including another QA MusicOS process were present. Automation-observed timings include accessibility/tool overhead; these are not exact first-paint or CPU timings.

| Native observed metric | Before | After |
| --- | ---: | ---: |
| Launch median | 2,105 ms | 1,934 ms |
| Launch p95, nearest rank | 2,860 ms | 7,516 ms |
| First Settings entry | 559 ms | 768 ms |
| Warm Settings median | 481 ms | 461 ms |
| Warm Settings p95, nearest rank | 546 ms | 479 ms |
| Main process RSS after visits | 197,072 KiB | 231,568 KiB |

The completed 7,516 ms launch is retained. Native capture/focus intermittently failed around that run, so its cause is not established. Thirty unsuccessful menu-script retries are excluded from timing samples and recorded in the raw report: the script mishandled an already-selected destination/open menu, then was corrected. Main-process RSS excludes WebKit helpers; retaining Settings deliberately trades some memory for state continuity. The small median improvements do **not** establish an overall launch-speed win because tail latency and first Settings entry worsened in this instrumented run.

Browser development fixture launches had medians 247.5 → 243.6 ms, p95 446.4 → 285.7 ms. Warm Appearance medians were 35.3 → 39.8 ms, p95 39.2 → 43.5 ms using the original script's quantile convention. First entries were approximately 819 → 823 ms. Both loaded 250 development script resources, with coarse JS heap around 26 MB. Resource counts/heap are proxies, not measured JavaScript CPU work. These fixture timings also do not prove a speed improvement. A controlled profiler trace for backend initialization, native artwork preparation, and JS CPU work before/after remains outstanding. Current backend snapshot/group/artwork/serialization component timings are retained in `backend-after.json` without an invented before comparison.

Evidence: `native-before.json`, `native-after.json`, `browser-before.json`, `browser-after.json`, and `backend-after.json` under `out/persistent-qa`. No 120 fps or touchscreen claim is made.

Final app: `target/release/bundle/macos/MusicOS.app`. Ad-hoc signed, with strict deep signature verification; this is not Developer ID signing/notarization. All 12 current UI JavaScript chunks were verified embedded.

- Executable SHA-256: `54246e43811152a65196bd71250443e62740d47a8dc809a2fc4239ada874cc21`
- Unchanged playback runtime SHA-256: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- Unchanged ONNX runtime SHA-256: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

Loaded the final build while main playback was paused at The Recipe, 276.4 seconds. The same track, position, 507 songs, 17 favorites, collections, and queue remained after relaunch. Subsequent user listening moved to 324.3 seconds, paused; that latest state was preserved. Final native audio reported Controls 1 / Applied 1, Room 100%, Depth 100%; wet frames advanced to 160,881 during real playback and stayed at that count while paused. Saved wet/depth were not changed by the final verification. The user subsequently answered **Yes, audible now**. This is human confirmation of an audible tail at that setting, not acceptance of every depth/music combination.

Native keyboard resizing increased mini-player width by 10 logical pixels. Artwork/queue switching preserved the footprint and queue entries; context actions and Return to MusicOS were available. Settings retained its Library category through main navigation. Earlier pointer resize attempts hit macOS `noWindowsAvailable`/capture failures. A later raised-window bottom-edge drag changed the screenshot height from 640 to 700 pixels at 2×, confirming one pointer resize. Broader pointer stress remains unverified. Native screenshots also confirmed all bottom tools remain in the same positions across views after the CSS repair. The silent speed-measurement app was closed after measurements; user testing profiles and their edits remain intact. The final loading-placeholder package was then loaded with playback still paused at 324.3 seconds, and the same counts, track, and collections were verified again.

## Preserved data and remaining acceptance

The user's metadata changes in MusicOS Update QA remain in its isolated profile. They were not copied into the main library. Its snapshot is retained at `out/persistent-qa/qa-edits-preserved.sqlite3`. File-writing/moving/forgetting tests use disposable fixtures; this task did not rewrite original music files. Main collections, favorites, source identities, and queue identities are retained.

Human confirmation of audible reverb at 100% room/depth is recorded above; broader listening quality at default and extreme settings remains a separate gate. Human motion/design acceptance, pointer resizing where macOS capture/input is unreliable, physical touchscreen behavior, 120 Hz packaged frame intervals, controlled startup/JS profiling, Windows, and Linux remain separate gates. Failed native input reports such as `noWindowsAvailable` are not counted as successful resize tests.
