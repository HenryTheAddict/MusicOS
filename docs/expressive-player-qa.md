# Expressive player and creation repairs — 2026-11-03

## Changes

- A native 320 ms palette handoff holds the displayed colors while matching artwork is pending. Rapid skips sample the currently displayed blend rather than restarting from an endpoint. Same-cover palette reuse remains in place. Paused color changes draw only through completion; hidden surfaces do not draw. Reduced Motion applies the target directly.
- The expanded player's theme surface and blob underlay interpolate independently of theme preferences. Flowing album colors are optional in Appearance → Now Playing, limited to the expanded player. Solid color is available. The existing flowing default is preserved. The low-resolution SDR scene keeps its broad forms/audio response and no longer overlays a separately changing blurred photograph. CSS color values are normalized before blending; this includes rgb and color(srgb) fallbacks.
- Lyric words fill progressively from their genuine start/end times and the acknowledged clock. Pause freezes the fill. Ordinary line lyrics remain whole-line highlights. A 32 px glow gutter (12 px in mini views), unclipped line ink, bounded glow radius, and restrained contrast shadow/stroke on visualizer lyrics keep active words readable without boxing them. The visualizer transport uses an edge-to-edge lower contrast fade instead of a rounded backplate.
- Acknowledged song-change notices show 44 px album artwork, title and artist, with a finite directional handoff. Song menus can open Now Playing. No automatic playback is issued.
- Description-free album heroes put quality and playback actions beside the text and level with the artwork bottom when there is enough content width. Narrow/long-content layouts retain natural flow.
- Visualizer options are compact. Appearance places their expandable section at the bottom. Existing icons, morphs and 44 px player targets are retained.
- Welcome setup keeps its footer reachable in short/square windows, follows live System appearance, and cannot save/dismiss during a folder-picker operation. Existing drafts, defaults and explicit imports remain intact.
- Add to playlist now supports creating a playlist with the selected sequence in one operation. A failed save retains the name and songs. Manual playlist creation keeps artwork optional, trims names and prevents blank-name submissions.

## Verification

- UI build succeeds; 226 UI tests across 72 files pass.
- 22 browser checks pass: shared player geometry at 480×480, 720×480, 1280×800; 80/100/125/150% scales; Light/Dark/blue/extreme themes; word fill/pause and glow space; notice artwork; compact options; description-free album alignment; square welcome layout; retained-dialog layering and keyboard focus.
- 18 visualizer library tests pass, including delayed palette arrival, rapid interruption, finite completion and Reduced Motion. Desktop preference compatibility/normalization test passes. Native desktop check and release build succeed.
- Screenshots inspected for the word treatment, small visualizer options, album footer and settled square welcome setup. Native silent Metal fixture confirms the lower fade and overlay geometry. The control tool could not activate the fixture's lyric data after a fullscreen Space transition, so live word timing is not accepted from that native screenshot; browser clock tests provide the timing evidence.
- Existing physical Automix quality, every Mini Player path, Apple authentication and physical trackpad acceptance are not expanded by this repair. No new claims about frame pacing or audio quality are made.

## Delivery

Use the isolated desktop branch. Preserve the latest profile and collections, previous signed app, primary OS work, and all eight embedded runtime/model files. Record hashes and install only after a fresh paused/stopped native check. Android and Wi-Fi schemas are unchanged; the only new saved setting is desktop player_background (flowing/solid).
