# Heart, lyrics scene and Settings repair — 2026-10-30

This repair stays on `codex/desktop-heart-preservation`. It changes desktop presentation and lyric editing only; playback commands, preferences, original audio, the Android companion and OS work retain their existing contracts.

## What changed

- The existing small heartbeat was limited to a 9% single peak, making it hard to see. It now has two visible peaks (24% / 13%) within the same cached BPM period and fixed hit area. Its tooltip reports analyzed BPM only. Pause, offscreen visibility and Reduced Motion still stop the beat. Unrelated renders do not restart it.
- The 640×480 scene now deforms three authored broad Bézier forms and a soft ribbon, using smoothed low/mid/high telemetry. Frame-independent attack/release smoothing keeps bass broad and upper bands tighter. Low-resolution canvas blur is used where available, with compositor blur in WebKit. A stable theme surface stays between color motion and text. Photos retain their SDR rendition path. No microphone, decoder, model change or new frame loop is introduced. A delayed first acknowledged clock can wake a settled renderer; closure unsubscribes it.
- Empty duplicate LRC cues no longer create tall blank gaps. A true instrumental break remains visible. The document is unchanged, and highlighting falls back to the preceding real line when a duplicate empty cue follows it.
- Settings categories use the active theme, Audio choices align with their labels, and equalizer/queue choices have shorter grouped layouts. Compact category navigation scrolls horizontally. The narrow lyric workspace removes outer navigation duplication, keeps an explicit Library settings return, and moves secondary source actions into the connected overflow. Its save footer no longer covers the editor. Focusing partially clipped text brings the editor into view immediately.
- Lyrics has one Words editor and a separate Timing view. Paste/file import opens only when requested; exports and reload remain in the connected menu. Song search, draft retention, candidate selection, save review, explicit file writing and History/Undo remain. Candidate rows close after selection. The cached-draft loading state is released correctly after interrupted loading.
- Editing Words also updates the timed-line text. Unchanged prefixes/suffixes keep their original word timings; changed lines clear incompatible word timings only. Added lines share the preceding time until stamped. Timing edits reconcile back into Words. Source attribution, revisions and unrelated metadata retain their previous behavior.

## Verification

- Production UI build passed. 180 unit tests passed, including text/timing reconciliation, duplicate cues and delayed-clock renderer wakeup/unsubscription.
- 17 WebKit interaction regressions passed for retained navigation and drafts, fixed player geometry, heartbeat BPM/peaks, editor-only changes, menus, idle controls and Reduced Motion. A separate focused small-window editor check verifies usable text space and return to Library settings at 480×480 and 720×480.
- 240 layout cases cover Appearance, Audio, Words and Timing at 480×480, 720×480 and 1280×800; all four scales; Light/Dark artwork, blue and extreme custom colors. No horizontal page overflow was observed. Visual inspection caught the initial narrow navigation/overlapping-footer problem beyond that geometry check; focused tests verify its repair.
- Three headless WebKit trials per version compare the installed `cea7806` scene with the repair under identical synthetic telemetry. Both had 95th-percentile frame intervals of 18 ms and no intervals over 33.4 ms. Callback scripting p95 was 0–1 ms before and 1 ms after, at the timer's coarse resolution. Paused canvases stayed unchanged after settling, and closed scenes retained no canvas. These are browser fixture measurements, not native display-refresh or physical listening acceptance.
- Local generated evidence: `out/lyrics-polish-2026-10-30/`. No real library lyrics or original audio were edited during automated checks.

## Delivery

The release manifest records the signed executable hash, source commit, private latest-profile checkpoint and unchanged eight embedded sidecar/runtime/model hashes. Installation requires a fresh native paused/stopped observation. Running playback is left untouched. Physical Mac trackpad feel, native display pacing and Android remain separate gates.
