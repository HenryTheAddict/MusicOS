# Visualizer coverage and album colors — 2026-11-03

The native surface now uses `makeBackingLayer`; AppKit owns its root-layer geometry and backing scale. The prior manual midpoint placement produced a quarter-window image. The covered shell also has zero opacity so a descendant cannot expose the Library icon by overriding visibility.

**Album colors** is available in the visualizer options and Appearance → Visualizer. It defaults off and persists through the existing typed visualizer preferences. It uses the current artwork's cached palette independently of the interface theme, saturation and tint. Recoloring happens only in the final native GPU presentation pass; preset feedback, simulation, blend state and beat timing remain original. Disabling it restores the exact original output. Black and white detail remain bounded SDR. Missing/invalid art retains original colors; obsolete results cannot tint another song. Reduced Motion recolors the still thumbnail without adding a render loop. Changing colors does not restart timed rotation.

Verification: UI build passed; **69 files / 216 UI tests** passed; **16 renderer tests** passed; **13 browser checks** passed across 48 size/scale/theme combinations. The new real Metal readback check verifies exact original output, palette application, preserved black/white, invalid-palette fallback and exact restoration when disabled. Hook tests cover disabled lookup and late album results.

The isolated, silent native fixture was inspected edge to edge in fullscreen and after returning to a 900×650 window. Native options exposed Album colors; enabling it recolored the running boxes effect using the fixture artwork palette, while controls, focus restoration and Escape continued to work. This fixture never opens the user's profile or audio output.

The signed repair executable SHA-256 is `c963717b849d5f30cea14f4f2b219925eff9f35432efb618c1daeb48c114e858`. All eight protected runtime/model/helper files match the previous installed build. The release manifest records the latest checkpoint, previous app and installed checks separately. Prior Apple/live audio acceptance gates remain explicit in the main QA report; this focused repair does not claim those have passed.

## Installed check

Installed on 2026-11-03 after a fresh native paused check at Romeo, 0.5 seconds. The latest profile was checkpointed and both prior signed app archives retained. Native fullscreen showed the repaired coverage with lyric overlays and no stray Library glyph. The user's subsequent Album colors selection is persisted as enabled; the live effect reflects the album palette. The user resumed and then paused during inspection; no playback command was issued by the agent. The 625 tracks, 46 favorites, manual collection rows and ordered entry fingerprint match the pre-install checkpoint. Release details and hashes: `releases/desktop-visualizer-repair-2026-11-03/release.json` in the primary workspace.
