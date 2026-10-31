# Immersive lyrics and playback controls — desktop verification

Implemented on `codex/desktop-heart-preservation`, based on desktop source a2908a1. OS development and Android presentation/sync schemas are unchanged.

## Delivered behavior

Lyrics fill the window with broad SDR artwork colors and a stable text contrast layer. Idle toolbar and transport are inert and release their layout space after 2.2 seconds. Actual pointer movement, touch, keyboard input, focus, seeking, and menus reveal or retain controls. Synthetic pointer movement caused by layout changes does not restart the timer. Recentring uses 180 ms translations without text/artwork scaling. Desktop expanded-player volume is removed. Fullscreen tracks the native state and restores the prior state on exit; all expanded-player frames and shadows disappear in fullscreen.

Lyric editing, finding, versions, and attribution use right-click, Shift-F10/ContextMenu, or touch long-press. The queue and all three mini-player densities share the lyric renderer, loading/rest indicators, follow behavior, and source clock. Mini-player selection retains native bounds and queue identity; lyric menus use the existing native expansion sequence. No new saved preferences or playback commands were introduced.

Play/Pause retains the authored compatible paths and exact endpoints at 360 ms, with interrupted reversals. Next/Previous use finite 180 ms directional feedback without delaying commands. The heart retains its beat animation across normal updates; acknowledged seeks explicitly reanchor phase. Meter-only updates preserve the source receipt time, fixing repeated interpolation resets. Musical rest markers use the supplied Song icon; loading uses three animated horizontal dots. Plain lyric punctuation remains unchanged.

## Sketch exports

Canonical source: document C8FFC865-CF40-43FB-AA71-0B45DBA1EF0C, BaseUi page B738EF46-A0CE-4FED-A379-446ECFF7C12A. `assets/icons/ui/source.json` records every layer ID, dimension, file mapping, and SHA-256. Duplicate screen instances and empty canvases were excluded. Full-frame white export-canvas backgrounds were removed; supplied strokes and geometry were retained. Playback shading remains theme-aware. Close glyphs use supplied tinyX geometry in a normalized optical viewport; its dots morph uses matching endpoints. Source Play geometry and actual-size app rendering were inspected.

## Automated checks

- UI unit suite: 184 tests, including source-clock anchoring, morph intermediate frames/reversals/endpoints, normalized dots/close endpoints, and completion cleanup.
- WebKit/Chromium interaction checks: 22 passed. Cover idle/reveal/focus/touch, lyrics context access, nested Escape, queue/mini selectors, unchanged mini bounds, finite skip feedback, fullscreen framing, editor drafts/Undo, lookup states, source timing, musical breaks, and heartbeat seek phase.
- 72 layout cases: no horizontal overflow, clipped transport, missing lyric viewport, or interactive hidden controls at 480×480, 720×480, and 1280×800, four interface scales, Light/Dark artwork, blue, and extreme custom palettes. Compact mini lyric clipping was corrected using measured content spacers.
- UI production build passed. Native release signature and protected-file hashes are checked separately in the release record.

## Performance comparison

Identical controlled WebKit scene workload against the installed-source baseline, three trials each: baseline/updated 95th-percentile frame intervals were 18 ms in all trials. Callback scripting p95 was 1/1/1 ms baseline and 1/0/0 ms updated (timer resolution limits). Intervals above 33.4 ms: baseline 0/2/0; updated 0/0/0. Every trial left a stable canvas after settled pause and no canvas after closure. This is a browser workload measurement, not a native display-rate guarantee or proof of a reproducible missed-frame improvement.

## Delivery and acceptance limits

Install only after a fresh paused/stopped check. Preserve the previous app and latest profile/library/collections/lyric revisions. The release record contains signed executable SHA-256, source commit, unchanged embedded runtime/model hashes, and post-load native observations. No physical trackpad gesture or physical playback synchronization acceptance is claimed by browser tests. Headphone switching, DJ transitions, display pacing, touch, and Android remain real-device acceptance gates. No playback is started for delivery verification.
