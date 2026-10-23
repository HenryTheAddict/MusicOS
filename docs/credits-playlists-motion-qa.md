# Desktop credits, playlist access, and morph polish — 2026-10-22

This is a desktop-only follow-up on `codex/desktop-heart-preservation`. OS source, playback protocols, database schema, saved collections, and embedded runtimes are unchanged.

## Changes

- Album credits show the lead artist and named contributor chips. Guest-heavy albums show two named credits and a complete, bounded artist menu. Names wrap; opening a menu and pressing Escape restores focus. Contributor portraits no longer obscure audio/play controls or require hovering to discover names.
- Saved manual and smart playlists have a rounded box below More, with their own artwork colors, open/shuffle controls, drag-to-add eligibility for manual playlists, and shared context actions. Sparklists remain available through Home and More. The sidebar scrolls at constrained sizes while library totals remain visible.
- Creation Plus/Close rotates one fixed pair of rounded strokes through 45 degrees over 140 ms. It reverses from the visible angle with constant stroke weight.
- Play/Pause retains its supplied endpoint silhouettes, theme shading, and full 360 ms morph. Two adjacent pieces of the play silhouette become the two pause bars; overlapping whole triangles have been removed. One compound fill prevents a shared-edge seam, and internal outlines become visible only as the bars separate. Reversals continue from current progress and take proportional remaining time. Reduced Motion switches directly.
- Superseded portrait-only/hover-label CSS and tooltip behavior were removed. Compact About previews remain beside header controls.

## Verification

- 119 unit tests pass (38 files), including split-contour separation, bounded widths, reversal continuity, cleanup, and the fixed-center creation cross.
- Final targeted browser run: 18 passed. Includes saved playlist persistence and context-focus restoration; 17 contributors with a long name; 720×480 and 1280×800 at 80/100/125/150% in Light/Dark artwork themes; menu keyboard access; sidebar scrolling; rapid reversal; Reduced Motion; mini-player retained queue, scroll, fixed controls, and 244×320 minimum.
- Additional desktop regression run: 15 desktop-consistency checks and 5 restored-motion checks passed, covering search, Settings, theme/heart geometry, mini-player 1×/2× alpha, row controls, and persisted time preferences. The old About test assumed the previous layout and attempted to open an album absent from Home; it was updated to use Albums and assert the requested compact header placement. Its eight-size matrix then passed.
- Intermediate-frame inspection caught an interior triangle seam that geometry-only tests did not. The final five-frame contact sheet is `out/rebuild-qa/credits-transport-frames.png`; the seam was removed before release.
- Initial failures in the retained logs concern the sidebar overflow, a modal-hidden test locator, and SVG serialization in a test. They were corrected and the final targeted suite passed.

Logs and screenshots are under `out/rebuild-qa/credits-*` in the isolated worktree. Native delivery and hashes are recorded in the release manifest and the delivery section below.

Human motion/design acceptance, physical display frame timing, touch, Windows, and Linux remain separate gates. No guaranteed refresh-rate claim is made.

## Signed delivery

- Desktop source: `34aba0e`; primary compatibility cherry-pick: `3ad3520`.
- Signed application executable SHA256: `e3809c940528e0a13b56d4fdbd55242f1f99a318e369d1aa2806cc2af31f8284`.
- Unchanged embedded audio executable: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- Unchanged embedded ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
- Ad hoc signed and verified with strict deep verification. Existing compiler warnings only; release build completed in 1m33s.
- Preserved the previously installed application, latest private preferences, and verified SQLite backups before replacing the paused app. No older preferences or queue snapshot was restored.
- Installed and loaded in the main MusicOS. On Track restored paused at 233 seconds; 507 songs, 26 favorites, and 12 saved collections remained. Mm and Poop appear below More. Opening Mm and its shared right-click menu was verified natively. User navigation interrupted the attempted native album check; the contributor layout/menu was verified in the browser and size/theme matrix instead.
- Evidence: `out/qa/os-foundation/credits-motion/`; retained package and previous app: `releases/desktop-credits-motion-2026-10-22/`. `release.json` records exact hashes and delivery status.
