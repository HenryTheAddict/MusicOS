# Desktop previews and iPhone polish — 2026-11-07

## Delivered behavior

- Recent songs and playlist suggestions have separate 30-second Preview and Play actions. Preview borrows the existing spare native deck, freezes the original source position and queue, and restores the previous paused/playing state. Automatic Automix preparation cannot interrupt an excerpt. Explicit transport actions supersede it. The preview polls only while open and stops on Mini Player/visualizer entry or unmount.
- Home uses compact filled recommendation rows, bounded artwork, shorter section spacing, two-line mix titles, and an explicit Preview mix action. Mix creation and saving remain separate from playback.
- The Automix planner accepts reliable measured structural changes aligned to real beats. It no longer requires the end of an overlap to be another structural change. Protected repetitions, confidence, tempo, drift, and duration constraints remain enforced. Queue arrangement considers musical compatibility in a bounded next-twelve-song window, preserving pins and queue-entry identities. Changing other DJ settings no longer disables an accepted transition.
- The shared lyric clock absorbs small telemetry jitter without repeatedly pulling word fills backward, freezes after stale telemetry, and immediately reanchors seeks, speed changes and track handoffs. Equal-start genuine words use the next distinct timestamp; overlapping voices keep real word ends. Layout measurements include width and font loading. The detached glow uses the phone player as its local coordinate root and preserves variable-font geometry.
- iPhone: Computer is inside Settings. Appearance, animation, Automix, Music Haptics and interaction-feedback controls persist. Home/Search/Library have finite content/icon transitions and restore scroll before painting. Scroll fades appear only at actual overflow boundaries. Downloaded-cover colors use the same bounded native SDR extractor as desktop rather than relying on readable WebKit canvas pixels.
- iPhone: supported native formats open directly; portable-codec hot paths are optimized in development builds, decoded copies are reused, and the next queued song is prepared off the main thread. A first uncached Vorbis/FLAC still prepares a complete WAV; this is not a streaming decoder rewrite and no measured physical cold-start speedup is claimed.
- iPhone: Wi-Fi transfers use pinned background URLSession downloads, temporary staging, SHA verification, scoped Keychain resume data, and retained collection identities/edits. ActivityKit/WidgetKit displays local sync progress. Phone Automix is explicitly a prepared two-second ending crossfade; it does not claim desktop phrase-aligned mixing. Apple Music Haptics uses supported recording evidence and system availability.

## Verification

- UI unit suite: 253 passed in 77 files. Desktop native library: 99 passed. Tauri app library: 237 passed, 7 intentionally ignored. The final native excerpt test also passed after adding the stale-plan guard.
- WebKit: 15 targeted Home, preview, lyric and layout checks passed after updating two stale expectations (removed setup tagline; closing menus become aria-hidden/inert immediately). Five Apple cache-flow checks also passed. Existing matrix coverage includes narrow layouts, interface scales and theme variants. These tests do not establish physical display FPS.
- Rendered native audio checks verified equal-power gains and exactly-once dominant-deck handoff. A real-recording, cached-analysis test completed an eight-beat blend without seeking: prepared, accepted, overlap active and incoming promoted. It ran muted and is not a subjective listening-quality pass.
- Read-only planner comparison over 672 cached recordings: 125 versus 1,113 qualifying musical plans among 450,912 directed pairs; maximum reported grid drift 49.57 ms. These counts demonstrate fewer unnecessary fallbacks, not improved listening quality.
- Fresh simulator harness: 28 checks passed. Covers FLAC, 256/320 Vorbis and AAC offline playback/seek/pause, duplicate queue entries, shuffle, favorites journal, manual playlists, lyric cache, system media information, prepared crossfade handoff, native SDR palette extraction, and a real pinned local HTTPS background download with SHA verification. The harness removes old results before launch and refuses non-fixture libraries.
- Installed Mac: started paused at the retained source position; library and collections present. Preview started and stopped, and the main song remained paused at the same position. Native window needed to be raised before WebKit/automation state updated; no workaround was shipped for this observation behavior.
- iPhone signed build installed and launched. Before/after profile comparison preserved tracks, collections, outbox, queue, duplicate entry identities, queue index and saved source position. Physical playback/lyrics feedback is pending.

## Remaining acceptance gates

- Audible installed-build Automix quality and full output-switch/interruption matrix.
- Physical locked/suspended-phone Wi-Fi resumption and Dynamic Island presentation; simulator background API checks do not establish these.
- Physical iPhone Music Haptics availability/feel and uncached playback latency.
- CarPlay and iCloud storage are not implemented in this repair. They need separate entitlement/provisioning work; no placeholder controls or cloud credential handling were added.

## Delivery

Work stays on `codex/desktop-heart-preservation`; OS checkout remains separate. A local delivery checkpoint contains the previous desktop app, latest desktop profiles, iPhone profile before/after, signed artifacts, test logs and exact hashes. The eight protected runtime/model files are byte-identical. The Mac update was loaded after a fresh paused check; the iPhone update followed the user’s explicit paused/unlocked confirmation. App startup does not begin playback.
