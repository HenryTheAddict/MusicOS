# Richer lyrics and Apple enrichment — 2026-10-31

Desktop branch: `codex/desktop-heart-preservation`. Android and OS work are separate.

## Implemented

- Canonical `.lyricsfile.yaml` lookup precedes legacy `.lyricsfile`, LRC, and embedded tags. Existing selections remain first. Unknown YAML extension fields survive import/save/export at document, metadata, line, and word levels.
- Safe TTML import/export includes explicit word timing, namespace handling, Apple media-clock timing, generic relative/sequence timing, overlapping voices, singer agents/groups, and retained original XML. DTD/external entities are rejected. Unsupported mixtures of untimed spoken text and timed spans report an error instead of inventing word times. XML annotations not rendered by musicOS remain in the retained original source; TTML export emits supported timing and singer data, while LyricsFile retains the source.
- A shared active-cue set highlights simultaneous voices; Lead is left, Second voice right, Both centered. Additional identities are retained as unassigned lanes rather than silently reassigned. Metadata provides singer names, voice assignments, line ends, canonical export, loss warnings for LRC, and existing revision/Undo protection.
- Gateway 502/503/504 failures use explicit Retry, throttling and Retry-After without poisoning no-match caches. Existing cached/local lyrics remain independent.
- Dedicated theme-aware Apple setup: Connect → Match library → Bring lyrics, independent capability states, cancel/progress, ambiguous recording choices, searchable matches, and review into Metadata as an unsaved draft. Existing dirty drafts win. Sign-in occurs in an isolated incognito Apple window, without remote IPC permissions or access to other browsers' cookies.
- Read-only Apple prototype: credentials only in system keychain; default-off startup; documented library access attempted before experimental web endpoints; sequential bounded pagination; existing local matches only; separate derived index and lyric cache; no audio downloading, Apple-library writes, metadata replacement, automatic scans or lyric publication.
- Optional motion artwork in album hero, expanded player and mini-player; grids remain still. One visible video owner, SDR/H.264-or-explicit-SDR variants at no more than 720 square, bounded descriptor cache, static fallback, hidden/reduced-motion/low-power handling with native macOS power-change events. Video is streamed through WebKit rather than copied into the library. Native power-change handling is implemented; physical platform acceptance remains separate.
- Skip/favorite feedback releases finished Animation objects explicitly; durations, geometry, heartbeat ownership and playback commands are unchanged.

## Evidence and gates

Evidence folder: `out/richer-lyrics-2026-10-31/`.

- Production TypeScript/UI build passed. 192 frontend unit checks and 196 native unit checks passed; 7 native live/platform probes remain opt-in.
- Native tests cover formats, revisions/Undo, canonical precedence, preserved extensions, gateway handling and URL/session guards.
- WebKit checks cover simultaneous cues, singer/end-time drafts, Apple demo connection and explicit sync, Metadata review handoff, context/keyboard access, retained queue/mini bounds, and finite animation work.
- Setup layout/contrast matrix: 480×480, 720×480, 1280×800; 80/100/125/150%; Light/Dark artwork, blue, bright-yellow custom and black custom. All 60 combinations passed before the final story-art square sizing refinement; narrow layout and functional checks were repeated afterward.
- Headless finite-animation checks observe creation and bounded completion rather than assuming a tool round trip occurs inside a 180 ms animation.
- Explicit public-site web-authorization probe passed without an account; token contents were never printed. Public catalog probe on HIT ME HARD AND SOFT returned no `editorialVideo` descriptor. That live art gate is unavailable for the tested response, not claimed supported.
- Authenticated Apple sign-in, subscription lyric retrieval, real library pagination, account reconnect, actual video rendering/decoding and power-state changes remain live acceptance gates. No successful demo or login is treated as proof of those capabilities. No new physical headphone/trackpad or Windows/Linux acceptance claim.

## Delivery

Build without preparing or replacing embedded runtime/models. Preserve the latest profile, collections, lyric revisions, current app and OS checkout; verify signed executable and protected file hashes. Install only after a fresh paused/stopped check. New account-derived Apple data stays in the desktop-owned provider store; credentials are excluded from normal files and release archives.
