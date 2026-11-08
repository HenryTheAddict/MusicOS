# iPhone touch and management repair — 2026-11-07

## Changes

- Disable the WKWebView pinch recognizer, constrain its zoom scale, and set a fixed-scale viewport. Dynamic Type still comes from the native text-size preference. This does not disable iOS system accessibility zoom.
- Replace page-wide masked/filter compositing with solid surfaces and scroll-edge gradient overlays. Keep the native webview opaque. Ordinary theme transitions use background/text/SVG color properties rather than registered custom-color animations. These changes address the reported pink flashes; a physical-phone visual check remains necessary.
- Stop rebuilding the full React phone view on every unchanged playback text-scale update. Search/library song lists and long queues render bounded initial batches, while playback and saving retain the complete sequence.
- Remove the bounded readiness-polling loop before local-file playback. AVPlayer owns readiness and reports errors through the existing item observer. Warm the next decoded file and its AVURLAsset metadata. Cold Vorbis/FLAC still uses a full portable decode into a reusable WAV: this pass does not claim streaming decoding or instant uncached starts. A pre-change physical diagnostic measured 521 ms preparation for one recording; no physical before/after speedup is asserted.
- Queue separates the current song from upcoming entries. Add songs through a searchable picker, remove individual entries, drag a handle to reorder, or use keyboard arrows. Clear upcoming requires a choice and preserves the current entry. Duplicate queue identities stay intact; edits do not implicitly start playback.
- Playlist editing separates Playlist and Add songs, keeps search and drafts, supports duplicate entries, drag/keyboard ordering and removal Undo, and keeps Save visible. Closing a dirty draft offers Keep editing or Discard. Saving an obsolete revision is rejected and retains the draft. Success counts in queue selection update only after acknowledgement.
- The mini bar has bounded artwork/text, stable controls, a thin progress rail, and swipe left/right to skip or up to expand. Pull down from the player header to dismiss, swipe artwork to skip, swipe a song to reveal Play next, and use a leading-edge gesture in a detail header to return. Sliders and icon targets retain their own gestures. Queue/playlist handles alone own vertical drag/autoscroll.
- Add public UIKit feedback for controls, navigation, pickup, destination changes and drops. One native feedback path coalesces near-simultaneous events and respects Interaction feedback. Music Haptics remains the existing recording/system-supported feature, not invented BPM pulses.

## Verification

- TypeScript phone checks and production frontend build passed.
- Five dedicated browser tests passed: queue identity/edit/add/clear; drag without playback; playlist search/duplicates/order/remove/Undo/save; draft cancellation; narrow/tall/landscape fitting and increased text size. Sizes include 320×640, 430×932 and 844×390. The tests verify the fixed viewport and reachable footer controls after finite entry motion settles.
- Browser visual inspection at phone dimensions covered the queue and playlist editor using isolated fixture data. Source artwork and icon geometry remain supplied assets. Browser fixtures are DEV-only and excluded from production behavior.
- Fresh native simulator harness: 32 checks passed, including offline FLAC/256 and 320 Vorbis/AAC, seeking and pause, Automix handoff/cleanup, durable favorites, duplicate playlists/queue entries, background pin/hash checks, palette extraction, preserving current entry while clearing upcoming, feedback deduplication/disabled settings, and stale playlist revision rejection.
- Signed development build installed and launched on the connected iPhone after an explicit paused/unlocked confirmation. Before/after library snapshots and exact signed artifact hash are kept in the delivery checkpoint.
- Physical gesture feel, haptics, disappearance of transient pink flashes and cold playback latency remain acceptance checks on the iPhone. No higher frame-rate claim is made from browser tests.

## Preservation

This is a phone repair on `codex/desktop-heart-preservation`. Desktop app, OS checkout, original media, playback protocol, Android schemas, Wi-Fi manifests and protected runtime/model bundles are unchanged. The previous signed iPhone IPA and the latest phone profile are retained. Background-sync credentials remain in Keychain.
