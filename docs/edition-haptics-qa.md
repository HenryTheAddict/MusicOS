# Edition-aware lyrics and trackpad feedback — 1 November 2026

Desktop branch: `codex/desktop-heart-preservation`. Android schemas, Wi-Fi manifests, original audio, embedded runtime and models are unchanged.

## Changes

- Apple edition preference defaults to Explicit for unknown files. Recognized clean/explicit tags take precedence in Automatic mode; Metadata → Lyrics offers Automatic / Explicit / Clean for each song. Apple recording selections remain authoritative.
- A shared resolver ranks already compatible title/artist/album/duration candidates. Multiple equally compatible recordings still require a choice. Ratings never relax recording/edition identity or the existing two-second tolerance. Missing/unrecognized tags (including advisory zero) are Unknown, independent of musicOS star ratings.
- Global policy, per-song overrides, cached rating evidence, and match provenance live in additive desktop-only tables in the Apple enrichment store. Rating reads skip artwork/properties and are reused until the original file identity changes. No tag rewriting or automatic library scan.
- Automatic/legacy links recheck lazily. Saved lyric revisions and externally edited sidecars stay authoritative. Unedited, app-managed Apple LyricsFiles can follow a newly chosen cached edition; older cached versions remain retained. Stale native responses and frontend sessions cannot undo a policy change. Local/cached words remain readable while rechecking or offline; LRCLIB remains independent.
- Apple Settings uses compact Connection / Lyrics & artwork / disclosure groups, supplied single chevrons, and “Save lyrics & artwork.” Existing sessions, caches, and transfer workflows remain intact.
- Tall player controls use one centered grid. Mini-player footer targets share density variables; liked hearts have no permanent selected-view backplate. Artwork stays square, labels truncate, and preset bounds are unchanged.
- Follow lyrics owns reserved space. Mini-player Follow sits beside song identification. Initial following positions immediately; later cue following retains its finite transition. At the compact minimum, a full current line remains readable.
- Shuffle uses compatible authored SVG strokes for 180 ms, exact supplied endpoints, continuous reversal, and immediate Reduced Motion states. No opacity replacement or sampled polygon geometry.
- General → Trackpad feedback provides master, hover-beat and interaction switches, enabled by default on macOS. Apple’s public NSHapticFeedbackManager predefined patterns run on the native main thread. Unsupported hardware/system-disabled feedback is a silent no-op. No audio callback work.
- A single hover scheduler follows acknowledged position, speed and reliable BPM/phase. High tempos skip whole beats to stay at or below four pulses/second. Exit, pause, track mismatch, hidden/inactive windows and stale telemetry stop pulses. Native feedback coalesces duplicate events and prioritizes interactions over pending beats; stale pulses are discarded.
- Discrete feedback covers drag pickup, valid destinations/reorder slots, successful drops, favorites, toggles and committed slider steps. Autoscroll, ordinary movement and seek dragging produce no continuous ticks. Haptics are independent of Reduced Motion.

## Automated validation

- UI build succeeds.
- UI unit suite: 201 passed, including Shuffle exact endpoints/intermediate frames/reversal/Reduced Motion, provider-session invalidation, retained offline lyrics, beat skipping and disabled feedback.
- Native suite: 214 passed, 7 pre-existing ignored tests. Includes real FLAC clean-rating evidence, preference persistence, ambiguous counterpart ranking, authoritative manual matches, managed/external sidecar preservation, and native feedback coalescing. The final cancellation change also passes its focused native test. Native suite runs serially because existing sidecar synchronization intentionally has one process-wide job lock.
- WebKit: 41 checks passed, followed by 7 targeted confirmation checks after final footer consolidation. Checks cover fixed transport coordinates, 480×480 / 720×480 / 1280×800, all four scales, artwork Light/Dark, blue, extreme custom themes, mini-player minimums, Follow layout, single disclosure chevrons, heart timing and resizing/focus behavior.
- Test reloads emit existing cancelled dynamic-module warnings during page teardown; assertions verify the subsequent visible application state. No claim of a console-clean suite is made.

## Delivery gates

Signed executable and protected runtime/model hashes are recorded in the release manifest. Installation requires a fresh paused/stopped observation, with previous app and latest profile/databases archived. Native post-load inspection verifies the saved playback state, new settings, control geometry and preserved collections.

A physical Force Touch check with fingers touching the trackpad is required for tactile acceptance. Automated/browser checks establish scheduling and cancellation, not how it feels. Live Apple verification remains account/provider-dependent; no login, secret extraction, whole-library refresh or music-file rewriting is required for this delivery.
