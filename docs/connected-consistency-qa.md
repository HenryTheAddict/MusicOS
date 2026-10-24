# Connected motion and desktop consistency — 2026-10-23

## Result and compatibility

Personal Home, its entry and collection headings now use the saved display name’s possessive label. Startup does not invent a name; unnamed profiles use “Your library.” Existing saved names are retained. The sidebar has one evenly padded footer outside the collection scroller.

Find has a connected example/meaning panel with supported literal syntax. Escape dismisses help before Find, outside clicks preserve the query, and focus returns to its opener. More, overflow, contributors and context menus share compact spacing, a joined anchor seam, rounded selection and group separators. Contributor overflow excludes visible chips, retains portraits and scrolls inside the window. Integrations has one heading and contrast-checked surface-derived copy.

Play/Pause now uses authored compatible cubic paths with the supplied Sketch endpoints, full-width outline and soft two-tone fill, replacing sampled contour matching. The fixed-footprint 360 ms journey reverses from its displayed progress. Plus and dots/close use normalized coordinates. Theme/clock renders do not restart morphs. Finite shared animation ownership coordinates 180 ms content/layout and 140 ms control/menu feedback with the retained 320 ms artwork transfer. Lazy pages reveal their resolved content; interrupted effects capture their current visual state and release animation/observer work. Reduced Motion removes movement/morphs. Search/Back share one reversible sidebar slot.

Expanded artist biographies become the left two-thirds column at 48 rem of available content width, with portrait/facts/playback on the right. Narrow layouts expand below the header. Text is never scaled; paragraphs, attribution and Read less remain available.

Appearance offers Standard (244×320) and Compact (220×160) with still previews using the same player presentation and current playing artwork. Previews are inert, have no playback clock and request no metadata. Compact uses 10 px padding, 6 px gaps, a single row of 28×36 px targets with a 36 px central Play/Pause target, a 32 px thumbnail, and readable 11 px secondary labels. Native and frontend bounds agree; session sizes are keyed by density. Temporary menu expansion is capped to the monitor work area; tall players open the scrollable menu above their controls when there is no space below. View switches retain size, queue DOM and scroll. Hidden main-window presentation is retained without rebuilding its JSX on mini-player position updates.

The only new saved preference is optional `mini_player_density`, defaulting to Standard. No database migration, playback protocol, OS source, model pins or embedded audio runtime changes are included. Sketch Home, Smart playlist and Plus exports were refreshed from the current masters; Play next/Add to queue already matched. Provenance is in the asset manifest.

## Discord artwork

Artwork lookup first reuses exact artist/album/edition matches in the bounded existing catalog cache, then uses the existing exact-album lookup. Public Apple artwork is validated; local/private images are never uploaded. Album-key and request-generation checks reject stale completions. Provider errors, missing covers, fallback and RPC acknowledgement are distinct diagnostics. A rejected image retries with the `musicos` application asset; RPC acknowledgement explicitly does not prove rendering.

Set up Discord contains the existing MusicOS icon, a download and the one-time instruction to upload it as Rich Presence asset `musicos`. Discord accepts uploaded asset identifiers or public image URLs; absent assets require application setup, not automatic local artwork uploading. Source: [Discord ActivityAssets documentation](https://discord.com/developers/docs/social-sdk/classdiscordpp_1_1ActivityAssets.html). Actual Discord client image rendering is a separate delivery gate.

## Verification and measurement

- 133 UI unit tests and 155 Rust library tests passed. UI production compilation passed.
- The full 41-case browser run passed, including the 40-combination matrix: 720×480 / 1280×800, 80/100/125/150%, Light/Dark/blue/artwork/extreme custom. Geometry, menu collisions, footer spacing, readable contrast, keyboard/focus behavior, contributor deduplication, retained Settings, queue/heart alignment and transparent 1×/2× mini-player corners were checked.
- Final focused reruns passed after fixing an Appearance rule that overrode preset label alignment and after deferring lazy-page entrances until content resolves. Midpoint/reversal/Reduced Motion tests verify exact endpoint weight, a stable footprint and no remaining morph work.
- Baseline executable before changes: `c61e2e0baad148824643deb7b903ea72fb97f14c2d5eba02a0c5b3b6833512ea`. A native process sample is retained. Instruments/xctrace is unavailable on this Mac, so native presented-frame intervals and missed-frame counts are not established by this check.
- Controlled headless Chromium drawing benchmark: seven runs of 2,000 drawings, median 50.6 ms before / 11.2 ms after (77.9% less drawing cost). Six visible morphs had p95 callback intervals of 16.7 ms in both runs; callbacks over 25 ms were 1/152 before and 0/153 after. These are small browser samples, not native display pacing or a 120 fps claim.
- The clock benchmark injects 20 Hz native-shaped playback updates into an isolated 90-entry queue. Baseline and updated builds retain zero queue rerenders across five two-second windows. Median task work fell from 527.21 to 307.97 ms and scripting from 428.01 to 182.92 ms; ten view toggles fell from 360.19 to 240.04 ms. Style/layout work changed from 11.73/7.79 to 14.93/10.16 ms per two-second window. This development-fixture comparison shows less total work, with a small absolute style/layout increase; it does not measure native FPS. Signed/native delivery is recorded below after completion.

Evidence is retained under `out/rebuild-qa/connected-consistency/`. Disposable browser fixtures were used for mutation tests. No original music files, metadata or saved mixes were edited. Human motion acceptance, actual Discord rendering, physical touch and Windows/Linux remain separate gates.

## Subsequent user refinements

The user requested a genuinely smaller Compact layout after reviewing the first preset: large artwork was replaced by a 32 px thumbnail and all six controls now share one row, at a 220×160 native/frontend minimum. Standard remains 244×320. Opening uses one finite 180 ms appearance; returning to main uses a reversible 140 ms exit before acknowledged native window restoration. Queue switches retain their independent stationary frame. Geometry tests wait for the entrance to finish before comparing queue positions.

Public macOS MediaPlayer integration now adds forward/back interval commands, shuffle/repeat requests, acknowledged shuffle/repeat state, and queue index/count metadata. A native Playback menu exposes playback, ±15-second seeking, shuffle, repeat and the current/next 20 queue entries, plus Open full Up next. Queue menu identities resolve current entry IDs on activation and reject removed entries; duplicate songs remain distinct. Menu structure rebuilds only when queue/index/failure state changes, and checks update from playback snapshots. This does not add a public macOS editable queue UI or modify the embedded playback protocol.

Vesktop is used for the presence check. Its existing arRPC and Quick CSS settings were already enabled; no plugins, credentials or privacy settings were changed. A reversible MusicOS-blue Figtree theme was applied at the user’s request, with the original CSS privately backed up. The initial full cyan wash was rejected by the user for colors/contrast. It was replaced with mode-aware roles: Dark uses the actual MusicOS blue palette’s computed background `#1e2631`, surface `#28313f`, and contrast-adjusted accent `#64a1ec`, with pale ink; Light retains pale blue and dark ink. Profile/composer defaults, overlays, menus and icons use the same role mapping. After Vesktop restarted, macOS repeatedly reported noWindowsAvailable even after foregrounding. The final theme is saved, but post-fix client rendering and human acceptance remain unverified; completion is not claimed. Theme source and opt-in installer are in `extras/vesktop/` and `scripts/theme-vesktop.mjs`.

## Signed native delivery

Loaded the update only after a fresh paused-state check and clean application quit. The latest profile JSON files, including `integrations-v1.json`, and a verified SQLite backup were preserved privately immediately before replacement. The previous packaged app is retained in the dated release directory. The updated app restored The Prime Time of Your Life at 151.4 seconds, paused, with 508 songs; later playback/track changes were user-driven and were not overwritten.

Final executable SHA-256: `19911d058c83dbf9b7e97b77763a61d4ef19b5888e82d9b6e74476c3ae95651b`. Ad-hoc signing and `codesign --verify --deep --strict` passed. Desktop source: `6a31c36` (primary compatibility copy `8da20fa`). The subsequently committed theme/documentation changes do not alter the packaged app.

Unchanged embedded resources:

| Resource | SHA-256 |
| --- | --- |
| musicos-mac | `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d` |
| ONNX Runtime 1.31.0 | `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920` |
| CLAP audio.onnx | `0cc6fa08e10644765b907c04bf945ababf97ad1632f03e665de2b37fb4a4a78d` |
| moods.json | `cb31d08809b7f60bb3f8dba142249e4682a32936a7686889e160ab0b1b37f732` |
| preprocessor_config.json | `9739f58296aa6f9ac18008fd0150fb2649bc554985fbde86d0a4041c882ac753` |

Native checks passed for the new Playback menu and its Open full Up next action, compact one-row presentation at 220×160, queue open/close at unchanged bounds, actions expansion/return to main, preserved pin state, and retained Appearance page. Standard was restored after checking Compact. Actual user playback continued through subsequent checks. Pointer/keyboard resize edge cases and native presented-frame timing are not newly established by this smoke check.

All six saved playlist/collection tables were compared row-for-row against the immediate pre-load backup and remained equal. Integration configuration bytes, appearance, source preferences, enrichment, artwork and format profiles were retained. The only interface serialization addition observed was the default `mini_player_density: standard`. No secret values were printed.

The final 40-combination browser geometry/contrast matrix passed again after the Compact follow-up. Four focused exit/compact tests passed, including immediate inertness and zero leftover exit animation work. Some earlier entrance-geometry checks sampled a moving window; those tests now wait for the finite entrance to complete before comparing static positions.

Vesktop’s live presence connection was acknowledged on the installed build. The current album reported no matching public artwork and fallback; the configured Discord application’s public asset list is empty, so `musicos` still requires the documented one-time upload. No claim is made that the missing asset rendered. Actual post-update matched artwork and final blue-theme rendering remain client verification gates.
