> Retired on 2026-10-16: src-desktop and its SDL/C++ launcher were removed. This document is historical.

> Legacy C++ verification record. This is not acceptance evidence for the React/Tauri rebuild. See [current rebuild verification](docs/react-desktop-qa.md).

# Settings and library detail QA

final result: passed

## Findings

No remaining actionable P0/P1/P2 differences in the agreed desktop scope. The default blue interface, album header/rows, artist portrait/about/statistics/album strip, three Settings tiles, and navigation controls render correctly.

Expected content differences: Sketch uses placeholder titles, solid cover placeholders, mock collection totals, and a Lossless claim. The implementation uses shared demo records, existing demo covers, calculated totals and durations, and “Demo audio.” The M83 portrait and back icon were exported through Sketch MCP. The Sketch document was left unchanged.

## Evidence and normalization

Source document: `musicosdesk.sketch`, document ID `C8FFC865-CF40-43FB-AA71-0B45DBA1EF0C`.

- Album source frame: `LibraryAbumSelected`, `FD255550-27E2-45C6-92C7-E4011D71B9A0`.
- Artist source frame: `ArtistSelected`, `1A4C5CFE-1317-4880-9986-D2029BE234F1`.
- Source visual truth: `out/library-review/reference/album.png`, `out/library-review/reference/artist.png`, each 720×480 pixels.
- Implementation: `out/library-review/album-720.png`, `artist-720.png`, and `settings-720.png`, each 1440×960 pixels at a 720×480 point viewport (2× Retina density).
- Full-view comparisons: `out/library-review/album-comparison.png`, `artist-comparison.png`. Implementation downsampled to 720×480 to match source density; blue theme, top scroll position, stopped playback.
- Focused comparisons: `out/library-review/album-detail-comparison.png`, `artist-detail-comparison.png`. Matched header/content crops placed together and enlarged for typography, crop, icon, and spacing inspection.
- Large viewport: `out/library-review/album-large.png`, `artist-large.png`, `settings-large.png`, `album-adaptive-large.png`, `artist-adaptive-large.png`; 2560×1700 pixels at 1280×850 points.
- All settings scales: `out/library-review/checks/settings-all-scales.png`; individual album and Settings captures in that directory for 80%, 100%, 125%, and 150%.
- Fallback/contrast evidence: `out/library-review/checks/missing-artwork.png`, `dark-artwork.png`, `light-artwork.png`.

This is a native SDL/Cairo application, so CSS dimensions and browser console checks do not apply. Native window capture and the Cmd+, Settings shortcut were also exercised. Primary interactions were checked by dispatching SDL mouse events in window coordinates through the app's inverse scale transform.

## Required fidelity surfaces

- Fonts/typography: existing Rodin family and weights retained; album title 32, artist heading 60, detail text/controls follow the source hierarchy. Long titles and names truncate without breaking UTF-8. Native Cairo antialiasing differs slightly from Sketch rasterization.
- Spacing/layout: source 200-point sidebar and 64-point content origin retained. Album cover 128 points, row height 48 with four-point gaps, artist portrait 192 with a 64-point radius. Headers, controls, about panel, album strip, and scrolling remain usable at both viewport sizes. Settings uses a wide tile over two tiles, with vertical stacking available for narrow content widths.
- Colors/tokens: original blue is unchanged by default. Adaptive colors use cached artwork palettes, light content surfaces, dark accents, soft static blobs, and a short crossfade. Tested black-on-light and white-on-dark theme colors meet a 4.5:1 contrast ratio for light and dark sample artwork.
- Image quality: source M83 portrait preserves its subject, crop, colors, and rounded mask. Existing SVG controls and exported back icon remain vectors. Demo album covers are intentional content replacements for Sketch placeholders; artwork pixels are not recolored.
- Copy/content: totals and durations derive from shared records; unknown biographies use an unavailable message. Demo quality is labeled honestly. Settings names and scale labels match the approved scope.

## Comparison history

1. Initial comparison found the sidebar back affordance used the wrong icon. Replaced it with the exported Sketch back arrow. Post-fix evidence: album/artist full-view comparisons.
2. Small Settings layout needed less vertical padding. Reduced tile and field heights for compact windows. Post-fix evidence: `settings-720.png` and all four scale captures.
3. Large album capture exposed two-digit track numbers colliding with Play. Reduced and centered two-digit numbers within the original number gutter. Post-fix evidence: refreshed `album-adaptive-large.png`.
4. Final full-view and focused comparisons found no further actionable P0/P1/P2 issues.

## Functional verification

`make -C src-desktop all test` passed. Focused checks cover preference round trips, Unicode names, whitespace/blank/invalid fallback, missing preferences, failed writes, restart loading, stable route history and scroll, grid selection retention, selected-track playback, previous/next, shuffle membership/state, artist sequences, overflow actions, Settings selector return behavior, theme source precedence including Devices, unreadable/missing portrait fallback, original-blue fallback, contrast, and all four scaled mouse targets. Cairo reports successful rendering in the interaction checks.

## Open questions and remaining test limits

No blocking design questions. Real library indexing, real-file playback, and playlist editing remain deferred as requested. Custom Cairo content does not provide native accessibility semantics; screen-reader behavior was not verified. Physical mouse automation had inconsistent coordinate reporting, so scaled hit-target acceptance is based on SDL event dispatch rather than that automation surface.

## Implementation checklist

- Completed: settings persistence, adaptive resolver/cache/crossfade, typed demo routes and records, detail browse/back/play flow, shared transport state, responsive scaling, Makefile checks, and visual comparison.
- Follow-up polish (P3): richer biography overflow treatment and a future native accessibility layer.
