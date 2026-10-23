# Connected motion and desktop consistency — 2026-10-23

## Result and compatibility

Personal Home, its entry and collection headings now use the saved display name’s possessive label. Startup does not invent a name; unnamed profiles use “Your library.” Existing saved names are retained. The sidebar has one evenly padded footer outside the collection scroller.

Find has a connected example/meaning panel with supported literal syntax. Escape dismisses help before Find, outside clicks preserve the query, and focus returns to its opener. More, overflow, contributors and context menus share compact spacing, a joined anchor seam, rounded selection and group separators. Contributor overflow excludes visible chips, retains portraits and scrolls inside the window. Integrations has one heading and contrast-checked surface-derived copy.

Play/Pause now uses authored compatible cubic paths with the supplied Sketch endpoints, full-width outline and soft two-tone fill, replacing sampled contour matching. The fixed-footprint 360 ms journey reverses from its displayed progress. Plus and dots/close use normalized coordinates. Theme/clock renders do not restart morphs. Finite shared animation ownership coordinates 180 ms content/layout and 140 ms control/menu feedback with the retained 320 ms artwork transfer. Lazy pages reveal their resolved content; interrupted effects capture their current visual state and release animation/observer work. Reduced Motion removes movement/morphs. Search/Back share one reversible sidebar slot.

Expanded artist biographies become the left two-thirds column at 48 rem of available content width, with portrait/facts/playback on the right. Narrow layouts expand below the header. Text is never scaled; paragraphs, attribution and Read less remain available.

Appearance offers Standard (244×320) and Compact (220×280) with still previews using the same player presentation and current playing artwork. Previews are inert, have no playback clock and request no metadata. Compact uses 10 px padding, 6 px gaps, 36 px targets and readable 11 px secondary labels. Native and frontend bounds agree; session sizes are keyed by density. View switches retain size, queue DOM and scroll. Hidden main-window presentation is retained without rebuilding its JSX on mini-player position updates.

The only new saved preference is optional `mini_player_density`, defaulting to Standard. No database migration, playback protocol, OS source, model pins or embedded audio runtime changes are included. Sketch Home, Smart playlist and Plus exports were refreshed from the current masters; Play next/Add to queue already matched. Provenance is in the asset manifest.

## Discord artwork

Artwork lookup first reuses exact artist/album/edition matches in the bounded existing catalog cache, then uses the existing exact-album lookup. Public Apple artwork is validated; local/private images are never uploaded. Album-key and request-generation checks reject stale completions. Provider errors, missing covers, fallback and RPC acknowledgement are distinct diagnostics. A rejected image retries with the `musicos` application asset; RPC acknowledgement explicitly does not prove rendering.

Set up Discord contains the existing MusicOS icon, a download and the one-time instruction to upload it as Rich Presence asset `musicos`. Discord accepts uploaded asset identifiers or public image URLs; absent assets require application setup, not automatic local artwork uploading. Source: [Discord ActivityAssets documentation](https://discord.com/developers/docs/social-sdk/classdiscordpp_1_1ActivityAssets.html). Actual Discord client image rendering is a separate delivery gate.

## Verification and measurement

- 132 UI unit tests and 152 Rust library tests passed. UI production compilation passed.
- The full 41-case browser run passed, including the 40-combination matrix: 720×480 / 1280×800, 80/100/125/150%, Light/Dark/blue/artwork/extreme custom. Geometry, menu collisions, footer spacing, readable contrast, keyboard/focus behavior, contributor deduplication, retained Settings, queue/heart alignment and transparent 1×/2× mini-player corners were checked.
- Final focused reruns passed after fixing an Appearance rule that overrode preset label alignment and after deferring lazy-page entrances until content resolves. Midpoint/reversal/Reduced Motion tests verify exact endpoint weight, a stable footprint and no remaining morph work.
- Baseline executable before changes: `c61e2e0baad148824643deb7b903ea72fb97f14c2d5eba02a0c5b3b6833512ea`. A native process sample is retained. Instruments/xctrace is unavailable on this Mac, so native presented-frame intervals and missed-frame counts are not established by this check.
- Controlled headless Chromium drawing benchmark: seven runs of 2,000 drawings, median 50.6 ms before / 11.2 ms after (77.9% less drawing cost). Six visible morphs had p95 callback intervals of 16.7 ms in both runs; callbacks over 25 ms were 1/152 before and 0/153 after. These are small browser samples, not native display pacing or a 120 fps claim.
- The clock benchmark injects 20 Hz native-shaped playback updates into an isolated 90-entry queue. Baseline and updated builds retain zero queue rerenders across five two-second windows. Median task work fell from 527.21 to 307.97 ms and scripting from 428.01 to 182.92 ms; ten view toggles fell from 360.19 to 240.04 ms. Style/layout work changed from 11.73/7.79 to 14.93/10.16 ms per two-second window. This development-fixture comparison shows less total work, with a small absolute style/layout increase; it does not measure native FPS. Signed/native delivery is recorded below after completion.

Evidence is retained under `out/rebuild-qa/connected-consistency/`. Disposable browser fixtures were used for mutation tests. No original music files, metadata or saved mixes were edited. Human motion acceptance, actual Discord rendering, physical touch and Windows/Linux remain separate gates.
