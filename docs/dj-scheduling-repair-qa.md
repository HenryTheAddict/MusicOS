# DJmode scheduling repair — 2026-10-21

## Fault and correction

An automatic two-second ending blend could be accepted at track start before the musical planner delivered its result. Accepted schedules correctly resist routine replacement, so this fallback prevented a later smart plan from taking effect. The native regression test reproduced that failure before the repair.

DJmode now waits until near the ending before invoking its standalone fallback. The Tauri coordinator also holds weak Smart-mode ending plans until preparation is due, allowing analysis to produce a better plan first. Confident musical plans and explicit Full Song plans can prepare immediately. EOF and failed-decoder recovery remain immediate; ordinary gapless prefetch is unchanged when DJmode is off. Lead times scale with the user's playback speed.

Queue identities, accepted-schedule protection, pinned order, manual navigation, preferences, schemas and desktop presentation are unchanged. Source commits: desktop `22c732f`, combined OS branch `c283040`. OS services and embedded runtime binaries were not rebuilt.

## Verification

- Regression first failed because an early safety blend prevented a later musical plan; it passes after the repair.
- 85 desktop service tests and 141 Tauri tests passed. Checks cover delayed fallback preparation, user-speed scaling, EOF recovery and provisional-plan dispatch.
- A muted native fixture probe submitted a delayed three-second overlap. The mixer acknowledged the exact request, activated it, and promoted the incoming deck in 9.19 seconds. Queue identities remained unchanged, no seek occurred, and incoming source position was 3.10 seconds at handoff.
- The fixture probe uses two generated WAV files and an isolated temporary database. It verifies scheduling and mixer execution, not subjective musical quality or every analyzed song pair.
- The production app bundle built successfully; the combined OS branch also passed the example compatibility compile check.

## Delivery

The locally ad-hoc signed app passes deep/strict signature verification. It is not notarized.

| Component | SHA-256 |
| --- | --- |
| Desktop executable | `80fe1f03874f00d7865813ed0984d3097aa921a906e74202cbb1f4f3c816ccce` |
| Preserved embedded musicos-mac | `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d` |
| Preserved embedded ONNX | `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920` |

The previous and updated bundles plus release record are retained in `releases/desktop-dj-scheduling-2026-10-21/`. Current preferences were checkpointed privately before replacement. MusicOS was closed while paused and reopened successfully: Borderline remained paused at 3:16, with 507 songs, 20 favorites and saved mixes present. No main-library playback, queue edits or DJ preference changes were used for testing.

Evidence: `out/qa/os-foundation/dj-scheduling/`. Listening with DJmode enabled on real queued songs remains an acceptance gate. The muted probe cannot establish that transitions sound musically good.
