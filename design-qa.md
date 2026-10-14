# React/Tauri Sketch fidelity and motion review — 2026-10-09

final result: passed

The old native build was replaced and reopened after the Mac became accessible. Album, artist and back navigation were inspected in the packaged WebView with the actual 1,282-track library and saved 125% interface size. This report passes the scoped visual correction and browser motion checks; it does not call the whole rebuild accepted or establish subjective native animation quality.

## Source and implementation

- Source: `musicosdesk.sketch`, document `C8FFC865-CF40-43FB-AA71-0B45DBA1EF0C`, inspected read-only through Sketch MCP. No flow transitions are defined on the library/album/artist frames; the interaction motion added here interprets the user's requested play reveal and sliding titles.
- Source visuals: `out/rebuild-qa/sketch-album-current.png`, `out/rebuild-qa/sketch-artist-current.png`, 720×480 PNGs; fresh read-only exports and MCP screenshots confirmed these frames.
- Implementation: `out/rebuild-qa/album-720.png`, `artist-720.png`, and their `1280` versions. CSS viewport and PNG dimensions are 720×480 and 1280×800, density 1. Native WebView screenshots use Retina density 2.
- Full comparisons: `out/rebuild-qa/album-motion-comparison.png`, `artist-motion-comparison.png`. Each places the reference and implementation together at equal density.
- Focused header comparisons: `out/rebuild-qa/album-motion-header-comparison.png`, `artist-motion-header-comparison.png`; source and implementation crops x=200..715/y=64..294 at equal pixel density.
- Additional captures: `album-{width}-{percent}.png` and `artist-{width}-{percent}.png` for 80%, 100%, 125%, and 150%.
- State: blue theme, top of the album/artist, stopped playback, development fixture records. Source placeholders and fictional totals differ from actual records intentionally. Source playlists are deferred in the approved plan; the sidebar instead contains working Favorites, music-folder and metadata tools.

## Findings and comparison history

1. P1: The running native app was still an older revision with oversized elements and a different header. Fixed by replacing and reopening the signed native bundle; the current header, colors, controls and compact geometry are visible in the native captures.
2. P2, fixed: approximate panel/row/sidebar/album-strip blues. Read the actual Sketch fill values and corrected the tokens: main `#00aeff`, border `#0018ff`, translucent track rows `#0034ff40`, sidebar `#cbf1ff`, inactive scope `#aae4ff`, personal surface `#54b9ff`, artist strip `#8ed1ff`.
3. P2, fixed: track Play was permanently visible on every row. Hover, focus, selection and playback now reveal the control while the title slides 36px. Rows keep their positions and heights; touch devices show controls continuously. Reduced motion makes the state change immediate.
4. P2, fixed after the combined first comparison: album track rows were 7px too wide; artist controls did not align to the portrait bottom; personal sidebar started 6px too early; artist strip padding and card widths differed. Corrected those dimensions and captured the revised screens.
5. Motion: page entrances use cancellable Web Animations on existing nodes, preserving focus, scroll and virtualization. Covers settle over 240ms; text and controls use a short stagger. Playback updates do not replay the page entrance. Menus have 160ms opening/120ms closing; card presses, button presses and play/pause changes respond with short transforms. No animated scrolling or continuous background movement.

## Required fidelity surfaces

- Fonts: bundled Rodin L/M/DB/EB/UB weights, source 32px album title and 60px artist title, explicit sizes; long names truncate/wrap within responsive bounds. The current Retina WebView album/artist captures were inspected; exhaustive native text and scale acceptance remains outside this correction.
- Layout: 196px sidebar, x=200 content origin, 128px album cover, 48px rows with 4px gaps, 192px artist portrait. Compact geometry stays at larger window sizes; interface sizing alone changes the scale. Native controls are reserved at the left of the overlay titlebar.
- Colors: actual fill values inspected through Sketch MCP. Default remains blue; adaptive artwork remains opt-in and retains its fallback resolver.
- Assets: bundled Sketch icons and exported M83 portrait; real covers remain colored and sharp. Source black cover placeholders are deliberately replaced with actual artwork.
- Copy: actual counts, durations and formats; unavailable biography/artwork states remain honest. No invented playlists or library records in production.

## Verification

- TypeScript/Vite production build passes; 8 frontend unit checks pass.
- 20 browser integration checks pass, including playback, route restoration, artists/genres, menus, all interface sizes, 10,000-song virtualization, theme fallback, preference persistence, reveal hit targets and reduced motion.
- Final focused recheck after the last artist-strip spacing adjustment: 5 checks passed, including screenshot capture, all sizes, compact geometry, reveal and reduced motion.
- In-app browser reviewed album/artist routes at 720×480. Console: no errors.
- Native replacement completed. Captures: `out/rebuild-qa/native-album-motion.png`, `native-artist-motion.png`, both 2560×1600 pixels at 1280×800 points, saved 125% interface size. Actual album artwork, asynchronously resolved verified artist portrait/biography, real counts and back navigation rendered.
- Native animation smoothness, continuous resizing and broader rebuild acceptance remain unverified; see `docs/react-desktop-qa.md`.

## Implementation checklist

- Completed: source token/spacing corrections, track reveal, detail entrances, card/control/menu responses, reduced motion, stable scrolling and focused regression coverage.
- Completed: reopen and inspect the newly built macOS app; actual library and saved preferences retained.
- Follow-up: subjective native motion review and the broader native acceptance checklist.


## Responsive artwork and selected SVG icons, 2026-10-09

The regular and virtual collection grids now share one responsive layout. Covers remain square, fill complete rows and keep captions aligned. ResizeObserver tracks the persistent outer grid; virtual row identities include geometry so resizing cannot reuse obsolete row positions. The 720px / 150% breakpoint retains two covers rather than one oversized image. Development-only 1,000-album fixtures exercise scrolling and repeated window resizing at 80%, 100%, 125%, and 150%.

The 45 selected layers in Sketch's BaseUi page were exported read-only (44 distinct icons; both RepeatON masters are identical). `assets/icons/sketch/source.json` records their layer identities. White canvas backgrounds are omitted for transparent UI icons. The Back icon now uses the selected vector. SVG IDs and internal references are scoped per instance, including duplicate names inside an export.

`svg-morph-engine.html` remains intact. Its contour sampling/alignment and appearing/disappearing contour approach were adapted in `ui-desktop/src/svgMorph.ts` and connected to play/pause, shuffle, repeat and heart state transitions. Transitions last 180ms, cancel on replacement/unmount, settle on the exact source vector, and finish immediately when reduced motion is enabled. Preparation is cached with a bounded cache; idle icons request no frames.

Verification: production UI build and 8 unit checks pass. All 24 browser integration checks pass. After the final narrow-grid breakpoint correction, both responsive-grid checks pass again. Captures: `out/rebuild-qa/grid-responsive-small.png`, `grid-responsive-large.png`, and `sketch-icons-playback.png`. The final macOS bundle built, bundled and passed codesign verification. After the user approved restart, the updated native app was reopened with the actual 1,282-track library and existing preferences retained. The responsive grid and imported icons are visible in `out/rebuild-qa/native-grid-icons.png`. Native playback subsequently resumed. A native resize interaction was interrupted by user activity, so continuous native resize and subjective morph smoothness are not claimed as verified.

Native traffic-light positioning is available through `src-tauri/tauri.macos.conf.json` (`trafficLightPosition`, currently x=18, y=23). Tauri exposes no diameter setting. This change keeps native buttons and does not change their size or position. A specific new position or an AppKit sizing implementation remains a separate decision.
