# Artwork color, lyric motion and playback feedback — 2026-11-03

Artwork palettes now include three raw swatches from a bounded 128-pixel sample. Deterministic OKLab clustering retains meaningful secondary hues, ignores transparent pixels and tiny colored specks, and keeps monochrome covers neutral. Visualizer and liquid-scene colors use these swatches rather than contrast-softened interface surfaces. Existing Light/Dark appearance formulas remain unchanged. Palette cache version 7 is regenerated from existing local artwork/renditions, with no network refresh or whole-library scan. The optional final GPU recoloring retains vivid highlights until the top of the range rather than whitening most of the image. Original preset output remains exact when album colors is off.

Lyrics stay mounted through cue changes. A finite 240 ms scroll follows the active voice group from its displayed offset; reversal, manual scrolling, hiding, Reduced Motion and unmounting release the frame callback. Full and visualizer lyrics are larger, use the bundled Figtree family, and change emphasis without scaling text. The visualizer keeps only the current and next group visible while preserving surrounding layout. Short windows use a bounded reading size.

The seek bar uses a transformed fill without trailing width interpolation. Pointer scrubbing freezes its measured geometry while the remaining-time label opens; cancellation does not seek. New queue-entry acknowledgements reveal chrome, run a finite 180 ms rail reveal, and show a compact song/artist notice above the stationary shared transport. Duplicate queue entries count as real handoffs. Meter/position/theme updates do not restart these effects. Play/Pause retains its existing authored 360 ms morph and acknowledged state.

Verification:

- UI build passed; **71 files / 221 UI tests passed**, including palette staleness, frozen seek geometry, cancellation, finite/reversible lyric following and notice cleanup.
- **14 browser checks plus the retained-modal regression passed**: all three window sizes, four interface scales and four theme families; invariant transport coordinates across views; retained lyric nodes, larger computed type, seeking and skip feedback.
- **Three perceptual palette regressions and nine provider regressions passed**, covering distinct colors, neutral/transparent covers, specks, cached original artwork and unchanged interface contrast.
- Real Metal readback passed: original output exact, recoloring applied, black/white endpoints retained, invalid palette safe, original restored exactly when disabled.
- Silent native fixture inspected with real Metal output, larger timed lyrics, cue seeking and Next acknowledgement/notice. It does not open the user profile or output audio. Native fullscreen visibility while an app is on another macOS Space is not a live playback synchronization test.

Signed executable SHA-256: `1e0b4e706ec209afd50e7270c2d7dd3ad2ab41a709b59a9cf40ac35f9b6f1d16`. All eight protected helper/runtime/model hashes match the previous installed build. Delivery records preserve the latest profile, ordered collections and previous app. Broader Apple, Automix listening and physical trackpad acceptance gates are unchanged; this repair does not claim those passed.

## Retained-modal correction

The first installed inspection caught an existing top-layer interaction: opening the transparent visualizer from an already-open expanded player exposed that retained dialog and its independent backdrop. Ordinary ancestor opacity cannot hide a modal in the browser top layer. The visualizer now explicitly hides covered expanded dialogs and clears their backdrops, preserving their state for restoration. The new regression opens the visualizer from a retained Lyrics dialog, verifies hidden pixels/backdrop and restores that dialog on exit. The initial installed candidate remains archived separately; the final signed correction has the hash above.

## Final installed verification

The layering correction was installed after the user paused and a fresh native check confirmed Good Morning at 5.7 seconds. The previous app and latest profile were checkpointed again. Entering the visualizer from an existing Lyrics player showed one visible lyric layer and one progress bar over the purple album-derived scene, without the old cover or dialog. Exiting restored the prior Lyrics player and its shared transport. The final native control read Play (paused); the agent issued no playback commands. The ordered collections and library counts match the checkpoint. The signed build and checkpoint are recorded in the primary workspace at `releases/desktop-lyric-motion-layer-2026-11-03/release.json`.
