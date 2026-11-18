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

## DJ AutoMix follow-up — 2026-11-18

The desktop implementation through `6717771` chooses an upcoming order and forecasts each handoff using the same planner. DJ mode has one on/off switch. Queue forecasts identify the exact outgoing and incoming occurrences, and the playback bar shows the predicted mix range. Once the mixer accepts a schedule, its timing takes precedence over the forecast.

The November 15 comparison used cached analysis for 96 tracks and evaluated 9,120 directed pairs. The final comparison reported 240 beat-matched plans and a mean overlap of about 5.0 seconds, compared with 220 and 3.5 seconds before that change. It reported zero incoming starts more than 50 ms after detected first audio and zero outgoing cuts over 12 seconds. Those are planner measurements. Audible quality still needs review using the production decoder and mixer.

The useful next step is to find the transitions people actually dislike and keep those pairs as regression fixtures. A higher match count alone cannot tell us whether the result sounds better.

### What needs improvement

- **Energy-based exits.** Check gradual fades, quiet breakdowns that return to a chorus, reverb tails, and abrupt endings. Record the energy envelope and selected cue beside the rendered audio. The sustained-drop check should avoid treating a temporary break as the ending.
- **Phrase and vocal overlap.** Current density scoring estimates how crowded a blend will be. It does not establish whether two vocal lines clash. Review long blends with vocals, percussion-only intros, and sparse outros, then adjust scoring against those examples.
- **Beat alignment.** Keep the incoming pickup audible while aligning later measured beats. Test half-time and double-time grids, local timing drift, and weak beat evidence. Report maximum phase error across the overlap as well as the selected tempo ratio.
- **Blend length.** Current limits allow up to 12 source seconds in Subtle and 16 in Creative when the measured arrangement supports it. Review the whole gain envelope, perceived loudness, and final note. A longer overlap can still sound rushed if the useful handoff happens in its last second.
- **Queue decisions.** Test the sequence of transitions across a set, including pins, duplicate entries, removed songs, and analysis arriving late. Explain why a song was selected. Verify that each prediction belongs to the current queue revision and that edits invalidate stale forecasts.
- **Failure recovery.** Delay or starve the spare decoder, seek during preparation, cancel during an overlap, and disable DJ mode with queued work still pending. Capture plan IDs, cancellation epochs, decoder generations, and source positions. A missed mix should recover to the same playable next entry.
- **Playback presentation.** Check marker position before scheduling, during the overlap, and after the incoming song owns the player. Scrubbing must work through the marked region. Check the letter animation, dark-mode contrast, and Reduced Motion during actual playback.

### Evidence to attach to the next fix

For each failing pair, save the source identities and analysis versions, queue-entry IDs and revision, planner decision, mixer acknowledgement, activation time, and decoder readiness events. Render audio through the production mixer and include a short account of what the listener heard.

Compare the old and proposed result on the same inputs. Check clipping, silence gaps, source positions, overlap length, pitch preservation, underruns, and CPU usage. Include unrelated pairs in the check so tuning one ending does not break another. Keep listening observations separate from automated measurements in the review.
