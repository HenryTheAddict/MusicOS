# Player motion, enrichment cache, and Sparklist improvements — 2026-11-01

Isolated desktop branch: `codex/desktop-heart-preservation`.

## Changes

- One shared PlayerTabs treatment in expanded player, queue panel and all mini-player presets. Selected and unselected labels use the same 500 weight and fixed hit targets. A finite 140 ms fill moves behind them; width and current transform are retained on interruption. No selected-label layout change.
- Acknowledged track changes animate artwork/text with a short 180 ms directional handoff. Playback actions remain immediate. Existing 360 ms authored Play/Pause endpoints and heart geometry are unchanged. Views enter with short directional motion; hidden outgoing content remains inert. Reduced Motion skips spatial effects.
- Real word timestamps drive a soft finite glow. Line-only lyrics do not invent word timing; pause removes active word animation. Current simultaneous voices retain their existing timing and alignment.
- Opted-in native background caching for signature-checked local Apple matches only. It continues when Settings closes, retains caches on cancellation, resumes an interrupted enabled job, and stops when finished. Cached credentials remain in keychain. Saved sessions are identified separately from freshly verified sign-in. Provider throttling/temporary failure pauses work rather than retrying indefinitely. Missing lyric/art outcomes are held for 24 hours.
- Successful lyrics remain in the provider cache; canonical LyricsFiles are synced after the batch while existing selections/externally edited sidecars remain authoritative. Only affected lyric caches are invalidated. No audio tags or unrelated metadata are changed.
- Static matched artwork is cached separately as bounded 800 px SDR JPEG with a 256 MiB budget. Original covers stay intact. Only missing UI artwork can use this fallback. Public CDN image requests receive no account credentials. Motion descriptors are prewarmed when supported; animated video itself remains streamed, with still-cover fallback offline. This does not claim offline video caching.
- Eight recent playlist headers remain the initial view. Once checked, unresolved entries are grouped by album/artist with song names/counts. Transfer still requires every entry to uniquely match an available local file, without silently dropping repeats.
- Sparklist v3 reuses expensive embedding scores instead of recomputing them during selection. Bounded diversity and adjacent-artist spacing operate on the primary artist (including featured-artist variants), while audio relevance remains dominant. Existing saved sequences, installed models and embeddings are retained. Creation shows real coverage, artist/album/song counts and operation-specific status. Idle/hidden creation pages stop status polling.

## Verification

Evidence: `out/expressive-cache-2026-10-31/`.

- Production UI build and full native/frontend checks.
- WebKit: stable selected-label widths, rapid tab changes, finite fill/skip work, genuine word glow and stopped pause animation, mini preset bounds, idle Sparklist polling, unsaved previews, Apple review handoff, cached transfer checks and 400-playlist top-eight behavior.
- Native: model relevance/diversity and normalized primary-artist spacing, album-needed grouping, provider/outage classification, recording-scoped cached covers, preserved original artwork/audio, and stopping without deleting caches.
- Automatic background behavior, real provider coverage and cached-file delivery need installed-build observation. Physical word/audio synchronization and subjective mix quality remain listening checks, not inferred from automated tests. Motion descriptors do not guarantee that a particular album has a supported SDR animation.
- Preserve latest profile, collections, user lyrics and previous app. Record signed executable and unchanged eight runtime/model hashes; install only after a fresh paused/stopped check. Android and OS work remain separate.

## Apple lyric error and automatic source repair

The installed provider cache contained 37 Apple lyric documents, but normal player loading did not consult it. The pictured High Life recording had no saved Apple catalog link. The manual action rejected that missing link and the UI rendered its structured native error using String(object), producing `[object Object]`.

Player loading now uses existing local selections/sidecars/tags first, then signature-checked cached Apple lyrics, then LRCLIB cache. Online lookup uses the connected Apple account automatically, including an on-demand catalog match requiring exact normalized title, artist and album with duration within two seconds and exactly one recording. It never scans the Apple library or changes local metadata. Missing/ambiguous matches and Apple failures fall through to LRCLIB; negative Apple attempts are held for 60 seconds. Manual Apple actions show readable errors. Disconnected users retain LRCLIB without a sign-in requirement. Frontend provider cooldown no longer blocks independent Apple work; native LRCLIB Retry-After remains authoritative.

A live LRCLIB exact-signature request for High Life returned HTTP 200, duration 202 seconds, with no synchronized lyrics during this check. This is a successful provider response, not evidence of synchronized lyric availability. LRCGET was never bundled or removed: musicOS uses LRCLIB directly. Current provider documentation was checked at https://lrclib.net/docs.txt and the LRCGET project at https://github.com/tranxuanthang/lrcget. No provider outage can be fixed by changing client text.

Automated checks: 193 frontend tests and 206 native tests passed (7 explicit live/physical checks ignored). Cached Apple lyrics load without credentials/network and saved local edits win. Native structured/nested errors render useful messages rather than object coercion. Final browser counts and installed observations follow in the release record.

## Installed delivery

The signed build was installed after a fresh native paused check (God’s Plan at 198 seconds). Opening Lyrics populated the installed player without starting playback. Background caching was observed progressing from 33 to 82 of 257 matched songs with Settings reporting that navigation can continue; completion is not claimed. The current provider cache remains separate from user revisions.

193 UI tests, 206 native tests (7 ignored explicit live/physical gates), and 21 WebKit cases passed. Preservation checks retained 550 songs, 40 favorites, 2 manual playlists and 6 entries, collection sidecars, current settings and user lyric revisions. All eight embedded runtime/model hashes matched. The unrelated existing primary-checkout guide edit was fingerprinted and retained.

Signed executable SHA-256: `85e669f64a3bc828c23ce042be5b94c5c1e503b42de8c377943038698033fde4`. Previous app and profile snapshot are retained in the release/checkpoint record. Native Apple lookup for the pictured High Life recording and physical word/audio alignment remain live acceptance checks; the verified cached/offline/default-source paths and readable error handling do not establish provider coverage for every song.
