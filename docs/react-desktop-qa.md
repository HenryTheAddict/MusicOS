# React / Tauri rebuild verification

### Audio tooltip clipping follow-up

The header audio tooltip now uses a manual HTML popover in the browser top layer,
with fixed coordinates clamped to the viewport and an upward fallback. Resize,
scroll and async specification changes update its position. Hover, keyboard focus
and Escape are supported. Regression checks pass at 720x480 and 1280x800;
album quality and all-scale detail-layout checks also pass. Native app reload
remains pending while the user is playing music. This build also includes the
three-bar playing equalizer, which becomes still on pause or reduced motion.

## 2026-10-09 — Menus, SuperCommand, source restoration and audio specs

Added reversible reference-folder removal/restoration in Settings > Library,
sidebar search/back with Cmd/Ctrl+K SuperCommand, real-file quality tooltips,
themed portal menus and working song/card right-click actions. Queue rows now
include artwork, a separate duration and current-track label. Fixed album track
number spacing, themed the footer and action buttons, and adjusted portrait crop
framing to 50% / 30% (a framing heuristic, not face detection). Custom SVG fill
and stroke colors inherit the theme while preserving their separate shades.

Discord presence resolves matching public catalog album artwork asynchronously;
local images are not uploaded. Exact artist/album matching and IPC asset payloads
have tests. Live Discord image delivery remains unverified.

Validation: 25 Rust tests, 19 frontend tests and all 36 browser integration tests
pass. Browser checks include all four sizes, 720x480 and 1280x800, large libraries,
keyboard menus/search, source restoration and tooltips. The release macOS app
built, bundled and passed strict ad-hoc signature verification. Reloaded the
paused MusicOS Verification app with its 1,282-track library intact; verified
native Cmd+K, real local search, Escape, sidebar back, pink artwork palette,
right-click Play/Edit metadata/View Artist menu, and actual FLAC specifications
(44.1 kHz, 16-bit stereo, 880 kbps). Capture:
`out/rebuild-qa/native-themed-context-menu.png`. The separate Design QA profile
and user browser tab were preserved. No new Windows/Linux or visible native
traffic-light checks were performed; the screen-sharing badge covers the latter.

Recorded 2026-10-08; updated 2026-10-09 on macOS (Apple Silicon). This report supersedes prior C++
`screenshot passed` reports. **Native acceptance is not complete.** The Mac was unlocked after an earlier interruption and native checks resumed. No Windows/Linux hosts
or physical MusicOS device were available.

## Evidence collected

- Rust desktop services compile without GPUI. 44 existing service checks pass,
  including library access, metadata overlays/matching, managed imports, source
  auditing, playback worker commands and collection ordering.
- 11 new Rust checks pass: stable collection identities/counts/order, preference
  round trips and fallback, C++ settings migration, sequence selection/shuffle,
  import cancellation, real referenced WAV indexing and lazy cover extraction,
  palette cache/contrast/missing images, obsolete network request cancellation, and
  actual FLAC decoder seeking/rewind.
- 8 frontend unit checks pass for history/scope restoration, theme precedence,
  dropdown keyboard selection, Escape/focus restoration, and outside clicks.
- 18 browser integration checks cover album/artist/genre browsing, no playback on
  browse, queue/previous/next, settings persistence and blank names, all four
  interface sizes at 720×480 and 1280×800, blue/adaptive/reset behavior, online/local
  separation, 10,000-song and large-queue virtualization, scroll restoration, and
  keyboard activation. Geometric checks guard artist-link/control overlap and the Sketch header/portrait composition.
- An explicit live-provider smoke check returned 59 Apple catalog album results,
  cached and decoded catalog artwork, verified Daft Punk's MusicBrainz identity,
  and resolved its Wikipedia biography and credited Wikimedia portrait.
- The macOS app bundle builds with the unchanged embedded preview sidecar.
  Release artifact: `target/release/bundle/macos/MusicOS.app`.

## Design review

Sketch document `musicosdesk.sketch` was inspected read-only through Sketch MCP:
`LibraryAbumLibrary`, `LibraryAbumSelected`, `ArtistSelected`, and `DevicesSelected`.
The album/artist layers, cyan palette, Rodin weights, icons, cover sizes, compact
composition, and scrolling rows informed the implementation. Browser captures
were reviewed at both requested sizes. Narrow 125%/150% album headers now place
artist links and controls on separate rows to preserve usable targets.

Actual screenshots and Playwright results are under `out/rebuild-qa/`:
`album-{width}-{percent}.png`, `artist-{width}-{percent}.png`,
`settings-{width}-{percent}.png`, and `playwright-results.json`.
These are frontend evidence, not native-window acceptance.

## Native checks performed

An isolated `MusicOS Verification.app` profile was used without resetting the
normal library. Native folder selection imported two generated FLAC tracks in
reference mode. Album browsing did not start playback. Embedded FLAC cover art
rendered, the artist link opened its real artist route, unavailable biography was
reported, and artist Shuffle populated the local queue and enabled shuffle.
Rodio playback progressed and next-track navigation selected the second file.
The original decoder failed FLAC seeking; the desktop-only decoder and length
fix now pass a real-FLAC regression test. Native retest in the packaged app sought to 6.9 seconds and Previous rewound
to zero without a playback error.

Name edits, Coverflow Colors and all four interface sizes responded in the
native window. Native Window-menu tiling resized and reflowed the app; continuous
border dragging was not established through the computer-use tool. Accessibility
exposed native close/minimize/fullscreen controls. macOS's screen-sharing pill
obscured their appearance during active control.

The bundled embedded preview launched from Devices and the native app changed
from Offline to Connected. Its native sync folder picker opened; the user
returned to Library before the review/transfer test was completed.

Following user feedback, the header proportions, compact album track rows,
artist title/portrait/album strip and Devices two-pane composition were corrected
against fresh read-only Sketch captures. The background now occupies a separate
frame so it does not cover panel borders. Browser captures confirm these changes;
the revised packaged version was reopened and its blue album page reviewed
in the native WebView. The saved name, 125% size and enabled artwork colors
were restored on restart; the test profile was then returned to 100%/blue.

## Second visual correction, 2026-10-09

The previous native design review did not establish fidelity. Larger windows
still enlarged the album cover, title, artist portrait and typography, and moved
the player away from the section selector. Those overrides are superseded by
`ui-desktop/src/sketch.css`, using the compact Sketch geometry at both window
sizes. At 100%, tests measure a 128px album cover, 32px album title, first track
row at y=240 and player starting at x=368 even at 1280px window width.
The default window is now the source frame's 720×480. Search typography, back
arrow, lossless indicator, compact two-row player options, Recently Added/Album
Name sorting, sidebar density and panel spacing were also corrected. Counts
remain accessible but no longer add an undesigned album-header text line.

Fresh browser screenshots at all interface sizes and 18 integration checks pass.
The Mac bundle was rebuilt. Native review of this specific revision is pending:
the Mac locked before the test app could be reopened. The currently running
Verification window can therefore still show the preceding revision.

## Required remaining native acceptance

- Real audible output confirmation, queue selection/removal, volume/repeat,
  and exhaustive previous/next sequences.
- Native continuous resizing, drag region exclusions, corner clipping/shadows,
  traffic light actions/colors, Retina text, and menus at both window sizes.
- Exhaustive scaled pointer-target checks in the revised native window at all
  four sizes; long-name/title and missing-artwork cases.
- Drag/drop reference/copy choice, cancellation/progress, metadata review, Devices
  sync review/transfer/cancellation and preview stop.
- Real device hardware transfer and Windows/Linux native builds/behavior.

CI now includes a React/Tauri build/test matrix for macOS, Windows, and Linux;
adding a workflow does not establish that those remote jobs have passed.

## Reproduce

```sh
npm run build
npm run test:desktop-core
npm run test:tauri
npm run test:ui
npx playwright install chromium
npm run test:e2e
MUSICOS_APP_DATA="$PWD/out/rebuild-qa/provider-data" cargo test -p musicos-tauri --test live_providers -- --ignored --nocapture
npm run package -- --bundles app
```

Native QA data is isolated under `out/rebuild-qa/verification-data`; generated FLACs
are under `out/rebuild-qa/music`. The normal app/profile was left running and was
not quit or reset during the isolated tests.

## Sketch motion correction, 2026-10-09

Fresh read-only Sketch layer inspection corrected exact surface colors, track-row widths, sidebar placement and artist control/album-strip alignment. Track play controls now reveal on hover, focus, selection and playback, with sliding titles and fixed row geometry. Detail entrances, card presses, transport state changes and menu opening/closing have short animations; reduced motion disables animation. Playback updates preserve scroll and do not replay page entrances.

The production UI and signed macOS bundle build. All 20 browser integration checks pass; after the final spacing adjustment, five focused geometry, screenshot, scaling, reveal and reduced-motion checks pass. Source and implementation full/header comparisons are under `out/rebuild-qa/*motion*comparison.png`; fresh source exports are `sketch-album-current.png` and `sketch-artist-current.png`.

The previous native app was replaced and reopened after the Mac unlocked. The user's actual 1,282-track library, name and 125% preference were retained. Album → verified artist → back navigation was inspected in the packaged WebView. Native captures are `native-album-motion.png` and `native-artist-motion.png`. Native motion's subjective feel and the broader acceptance checklist remain unverified. The scoped visual report is `design-qa.md`; the older C++ report was preserved in `docs/legacy-cpp-design-qa.md`.


## Responsive artwork and selected SVG icons, 2026-10-09

The regular and virtual collection grids now share one responsive layout. Covers remain square, fill complete rows and keep captions aligned. ResizeObserver tracks the persistent outer grid; virtual row identities include geometry so resizing cannot reuse obsolete row positions. The 720px / 150% breakpoint retains two covers rather than one oversized image. Development-only 1,000-album fixtures exercise scrolling and repeated window resizing at 80%, 100%, 125%, and 150%.

The 45 selected layers in Sketch's BaseUi page were exported read-only (44 distinct icons; both RepeatON masters are identical). `assets/icons/sketch/source.json` records their layer identities. White canvas backgrounds are omitted for transparent UI icons. The Back icon now uses the selected vector. SVG IDs and internal references are scoped per instance, including duplicate names inside an export.

`svg-morph-engine.html` remains intact. Its contour sampling/alignment and appearing/disappearing contour approach were adapted in `ui-desktop/src/svgMorph.ts` and connected to play/pause, shuffle, repeat and heart state transitions. Transitions last 180ms, cancel on replacement/unmount, settle on the exact source vector, and finish immediately when reduced motion is enabled. Preparation is cached with a bounded cache; idle icons request no frames.

Verification: production UI build and 8 unit checks pass. All 24 browser integration checks pass. After the final narrow-grid breakpoint correction, both responsive-grid checks pass again. Captures: `out/rebuild-qa/grid-responsive-small.png`, `grid-responsive-large.png`, and `sketch-icons-playback.png`. The final macOS bundle built, bundled and passed codesign verification. After the user approved restart, the updated native app was reopened with the actual 1,282-track library and existing preferences retained. The responsive grid and imported icons are visible in `out/rebuild-qa/native-grid-icons.png`. Native playback subsequently resumed. A native resize interaction was interrupted by user activity, so continuous native resize and subjective morph smoothness are not claimed as verified.

Native traffic-light positioning is available through `src-tauri/tauri.macos.conf.json` (`trafficLightPosition`, currently x=18, y=23). Tauri exposes no diameter setting. This change keeps native buttons and does not change their size or position. A specific new position or an AppKit sizing implementation remains a separate decision.

## 2026-10-09 — Figtree, MusicOS branding, smoothing and Labs

The packaged macOS app was rebuilt with the custom protocol, ad-hoc signed, verified, and reopened after explicit restart approval. The normal library still shows 1,282 tracks and four music folders; Henry, 125% and Coverflow Colors off were retained. Native AX confirms Figtree Settings content, MusicOS Labs, Discord off, Last.fm off, disabled Last.fm switch before authorization, and an inline Application ID error for invalid Discord setup. Native capture: `out/rebuild-qa/native-figtree-smooth-library.png`.

The existing 24 browser checks pass with named appearance switches after adding two opt-in switches. Separate new browser checks cover Figtree loading and balanced parentheses, MusicOS branding, Labs validation/enable/disable without storing secrets in JavaScript, finite reflow and reduced-motion behavior. Frontend unit suite: 12 pass including progress fill/clamping, drag draft surviving playback events, single pointer-release commit, visible seek failure, and Sketch smoothing control points. Rust suite: 18 pass including Last.fm signature and listening-time accounting, atomic integration config, and Discord IPC handshake/ping/ack/frame bounds. Live provider accounts have not been connected; see `docs/musicos-labs.md` for beta setup and limits.

The user explicitly authorized changing the Sketch font. All 258 text layers were changed to Figtree through Sketch MCP, preserving sizes, weights and layer frames; main library, album, artist and device frames were visually checked after conversion and the document saved. This supersedes earlier read-only-source restrictions only for this requested typography edit.

## 2026-10-09 — Artist overview, depth, Settings categories and artwork fallback

Fresh read-only Sketch inspection supplied the inset shadows: the player uses a 0/2/4px black-at-25% inner shadow; library shortcuts use 0/1/2px; the personal library panel uses 0/2/0px, its original blue fill and blue outline. The sidebar now uses compact source-like navigation and content-height personal tools rather than a large empty panel. Artist portraits use the source 64px radius and 0.6 corner smoothing. Short artist names remain readable above the portrait.

Artist portrait hover/focus reveals actual library genres, counts and release years with finite staggered transitions. Clicking pins the notes, Escape dismisses them, and reduced motion removes transitions. Verified portrait attribution appears when available; missing biographies and credits are not invented. Biography expansion stays within its scrolling surface. Album and artist cards no longer rotate their sidebar icons or use exaggerated hover transforms.

Settings has keyboard-accessible Appearance, Audio, Integrations and General tabs, restoring the chosen tab during the session. Audio controls the authoritative playback volume and reports the actual backend. General contains the name setting; integrations remain opt-in. Coverflow Colors now tries the same lazy embedded artwork used by visible covers, falls back from unreadable portraits to artist albums, and returns blue only when no usable artwork exists. Palettes are reused and artwork invalidation clears the cache. The palette also reaches the shell and sidebar rather than just the detail background.

The native AppKit command centers the real macOS traffic lights in the rendered titlebar at each interface size, preserving their native size, colors and actions. The static Tauri traffic-light inset was removed because TAO reapplies it on redraw, overwriting the dynamic placement. This supersedes the earlier note that traffic-light placement was unchanged. Windows and Linux keep platform decorations; their behavior has not been verified on this macOS host.

Checks: 18 Rust unit checks and 17 frontend checks pass. The browser suite's initial compact geometry expectation exposed the intentional short-title adjustment; its corrected assertion verifies that the heading stays above the 192px portrait and the album strip stays below it. All other 28 browser checks passed, including all four interface sizes, artist notes, reduced motion and Settings tab/volume behavior. Final consolidated browser and native results are recorded below after verification.

Final consolidated checks: all 29 browser integration checks pass; all 17 frontend checks and 18 Rust checks pass. The macOS release with embedded frontend built and passed ad-hoc codesign verification. A separate `MusicOS Design QA.app` ran against the isolated two-track database. Native Settings tabs, real-cover palette changes, album → artist navigation and pinned artist notes were verified in its packaged WebView. A native bottom-right edge drag resized the window from 1280×800 to approximately 1166×746 logical pixels and the layout reflowed. Captures: `native-settings-artwork-colors.png`, `native-artist-notes.png`, and `native-artist-resized.png`. The QA app was then closed; the user's playing normal app was not quit. The native screen-sharing badge obscured traffic lights, so their final visible alignment, hover colors and button actions remain unverified. The updated production bundle is ready; replacing/restarting the user's playing app awaits their restart choice. A final singular/plural caption correction followed the native capture and is included in the final signed build.

Subsequent native update: the normal app became idle (`Nothing playing yet`), removing the reason to defer its restart. The signed final build replaced the normal `MusicOS Verification.app` and reopened with the 1,282-track library and Henry preference intact. User activity then resumed real-file playback and opened Daft Punk. Native AX confirms the new artist notes, actual genres/counts/year range, biography expansion and verified photo credit. `native-updated-artist.png` shows its portrait-derived palette in the user's real library. This supersedes the pending-restart note above. No further native controls were manipulated while the user was actively browsing and playing music.

## 2026-10-09 — Library cleanup and manual batch metadata

Implemented the screenshot follow-up: removed the visible Artist notes label
while retaining portrait hover/focus/click and attribution; moved branding to
the titlebar beside native traffic lights (icon only in compact windows);
removed Discover from navigation and route restoration; separated Back from
library categories; compacted the personal library panel and moved its management
links into Settings > Library. Library controls use intrinsic sizes. Album-strip
captions wrap within separate, fixed card widths rather than running together.
Search focus has finite motion and reduced-motion behavior. All song surfaces,
search and management controls now follow the resolved palette. Extraction uses
area-weighted chroma bins instead of one maximally saturated pixel; cache version
4 prevents reuse of old results. Light/dark/vivid contrast checks still pass.

Batch metadata supports filtered virtual selection, explicit changed-field
checkboxes, numeric validation and single-song title editing. Rust validates
all IDs before an atomic manual-override transaction. Existing tags and unchecked
fields remain unchanged. Track overflow opens the editor with that song selected.
Navigation reconciliation skips collections removed by a metadata regroup.

Verification: 20 Rust checks and 19 frontend checks pass. In the 31 browser checks,
29 initially passed; two retained obsolete header-position/sidebar-brand assertions.
Both passed after asserting the requested new brand placement. The batch editing
and search/theme checks passed again after final navigation/layout refinements.
The signed packaged app's isolated two-track library saved a batch genre edit
through the real command bridge. Read-only SQLite inspection confirmed both
`user` overrides and unchanged titles/artists/albums. Native capture:
`out/rebuild-qa/native-batch-metadata.png`. Browser design capture:
`out/rebuild-qa/library-polish-artist.png`. Native traffic-light visibility during
screen sharing and Windows/Linux behavior remain unverified.

The final signed build was installed into the paused main MusicOS Verification
app and reopened with the existing 1,282-track library and Henry preference.
Native artist browsing confirms a separate Back to Artists control and unchanged
Artists category. Final native capture: `native-library-polish-final.png`.
The user began importing folders and playing music in the separate QA app during
verification (its library grew to 395 songs). That process and its profile were
left intact; it was not quit or replaced while playing. Only the main paused app
was restarted. The QA copy predates the final navigation reconciliation and small
selection-list height refinement; the main app and release bundle include both.

### Interaction feedback — October 10, 2026

- Removed legacy absolute positioning from collaboration credits. Native QA with the copied local library displayed Kodak Black, Offset and Travis Scott as separate links. Browser checks cover compact layout at 80/100/125/150%.
- Audio quality popovers dismiss on mouse leave, outside pointer press, window blur and Escape. Pointer focus no longer pins the tooltip; keyboard focus remains supported. Browser interaction checks pass.
- Normal Settings entry selects Appearance; explicit Audio and Library management shortcuts retain their destination.
- Internal drag drops with no native file paths are ignored before import inspection. Native reorder verification remains pending because the isolated QA app was being used during inspection.
- Creation buttons inherit the active palette with contrast text. Native red-album palette inspected. Exact collection-row Shuffle button exported from Sketch without document changes, including its smoothed white-to-green background.
- Failed saved cover paths fall back to embedded extraction. Repaired paths use a cache-busting artwork revision. Tests cover a repaired same-path cover and honest absent-artwork fallback without repeated processing.
- UI checks: 25 unit tests and 3 targeted interaction browser tests passed; queue checks passed separately. Frontend and native debug builds passed. The final artwork URL/cache-busting and transparent base shuffle-icon correction were built after native inspection; the running QA app was not restarted while in use.

### Card, contributor and control refinement — October 10, 2026

- Album grouping now uses the primary credited artist; guest credits (`;`, `&`, featuring/feat/ft) retain individual artist membership without splitting that artist's album. Previous raw-credit album IDs remain route aliases. Distinct primary artists with the same album title remain separate. This is a grouping heuristic until explicit album-artist metadata is available.
- Last-browsed card paint removed. Only the current playback album/artist gets a small artwork-level playback indicator. Long card titles and contributor labels wrap; virtual grids and track rows measure their content height. Album song rows show individual contributor links instead of empty separators.
- Create playlist uses a solid palette-derived surface; only Create Sparklist has the gradient edge. Hover controls stay transparent without new container borders. The base ShufflePlay SVG's exported white canvas was removed; the separate designed collection shuffle control retains its intended background.
- Artist portraits show a blurred artwork backdrop and two-square loading symbol until portrait lookup/image loading finishes. Reduced motion disables its rotation. Missing portraits retain an honest fallback.
- Queue and lossy quality use their proper icons rather than the File icon. SuperCommand's sidebar shortcut is a command glyph; search excludes unavailable local tracks, labels rule/audio/manual collections accurately, clamps changing results, and restores opener focus.
- Album descriptions resolve only from one applied MusicBrainz release-group identity to linked Wikipedia/Wikidata, with source attribution and cached text. No speculative name matching or invented copy. Expand/collapse and absent-description checks pass; provider request success with a verified real album remains unverified natively.
- Validation: 53 Rust unit checks, 27 frontend unit checks, five interaction checks, three designed-collection checks, two compact queue checks, SuperCommand and playback/large queue checks passed. Two outdated desktop tests were updated to the current creation modal and truthful fixed-crossfade label, then passed: queue reorder/clear and playlist/queue layout at 720×480 and 1280×800 at all four scales. Large fixtures (10,000 songs and 1,000 albums with resize/scaling) passed; Vite logged one ResizeObserver delivery warning in the large-grid run. Frontend and native custom-protocol debug builds pass.
- Native QA was playing Breathe Deeper when inspected. This latest build has not yet replaced the running QA process; restart approval was requested to avoid stopping listening without notice. Windows/Linux unverified. Musical two-deck DJ runtime integration and production audio Sparklist model acceptance remain outstanding, as described in the earlier release notes.
