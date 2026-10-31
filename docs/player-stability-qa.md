# Player stability, mini-player and back-swipe repair — 2026-10-31

Desktop-only work on `codex/desktop-heart-preservation`, based on d64a426. No playback protocol, saved preferences, Android schemas, original audio, or embedded models/runtime changes.

## Changes

- Current song stays on the left while Lyrics and Up next occupy the same right-hand slot; narrow windows stack these slots. The queue no longer replaces artwork. Hidden/visible chrome has the same song geometry, with finite opacity/short control motion rather than content relayout. Current lyrics target the actual vertical midpoint. Artwork endpoints remain bounded, with one 180 ms view handoff.
- The progress bar stays visible. Near-pointer/focus feedback shortens its rail horizontally over 180 ms to reveal elapsed/remaining time; seek targets and transport stay vertically centered. Now Playing, expanded player, and mini-player share this presentation. Saved time visibility remains respected.
- Header controls use equal 44 px targets with explicitly centered SVG viewports. Fullscreen has a descriptive tooltip. Context-only triggers are hidden from the accessibility tree while their region retains right-click/keyboard access.
- Lyrics hide native scrollbars while retaining scrolling and keyboard access. Upcoming lines progressively fade by cue distance; manual scrolling removes that attenuation until Follow lyrics is selected. The background combines blurred SDR artwork with broader, damped color forms and the stable contrast veil. No background animation runs after pause settles or closure.
- Favoriting produces an acknowledged 420 ms heart squash/turn/pop with a dissolving ring and eight small sparks. Its animation layer is separate from BPM motion; switching tracks does not trigger a false favorite celebration. Reduced Motion suppresses the burst. Favorites now has Play and Shuffle actions, using the available filtered favorite IDs.
- Standard mini-player uses one transport row to free space for a larger square cover. Compact gets a 40 px cover; Wide retains its own footprint. View choices are quieter, presentation switches use a finite 180 ms handoff, inactive compositions stop subscribing to the playback clock, and all preset minimum bounds remain unchanged.
- Interactive Back hands off in the same rendering transaction, restoring the saved scroll before releasing the preview, rather than holding the departed page for two additional frames. The inert previous page has a short 24 px parallax reveal. Fully completed drags commit immediately; shorter finish/retraction durations are bounded. Velocity uses recent samples, and zero-distance native lift events retain the deliberate flick speed instead of forcing a retraction. Hidden images no longer misalign snapshot artwork. Normal Back retains its separate artwork transfer.

## Verification

UI unit suite: 184 passed. Focused browser suites cover original player palette/heart/transport tests, revised idle geometry, source timing, metadata lyric drafts, queue/mini selectors and bounds, centered header targets, favorite burst completion/Reduced Motion, favorite shuffle, and finger-following/reversal/momentum/full-distance back handoff. 44 interaction cases passed across the final full suite and the added zero-distance native-release regression. The last targeted flick/layout/favorite rerun passed all seven cases.

132 WebKit size/theme states passed: 480×480, 720×480, 1280×800; four interface scales; Light/Dark artwork, blue, and extreme custom colors; visible/idle lyrics and queue views. Nine mini-player preset/view screenshots were inspected, including all three native minimum sizes in browser fixtures. No unexpected overflow, missing lyric/queue viewport, or transport clipping was measured.

Scene comparison against installed-source baseline, three identical trials each: frame p95 18 ms in every trial, zero intervals over 33.4 ms. Scripting p95 1/0/1 ms baseline and 1/1/1 ms updated (timer resolution limits). Every trial stabilized after pause and removed its canvas on closure. This is a controlled browser comparison, not native display pacing or physical trackpad acceptance.

Physical trackpad feel and audible lyric/BPM synchronization require real-device acceptance and are not claimed by synthetic wheel tests. Native observations and any remaining gates are recorded separately. Install only after a fresh paused/stopped check; preserve previous app, current profile, collections and lyric revisions. Do not stop running user playback for testing.
