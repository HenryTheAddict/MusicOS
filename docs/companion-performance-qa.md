# musicOS companion, library and resize repair — 2026-10-29

## Scope

Isolated `codex/desktop-heart-preservation`; OS checkout unchanged. Preserve installed runtime/models, preferences, collections, original audio and SDR photo rendering. Integrations remain a later pass.

- Desktop navigation remains at 720 px across all interface scales. The scale-aware compact threshold is min(720 px, 40 rem). A medium desktop header retains native-control clearance and gives playback the remaining width. Removed the obsolete enlarged-scale rule that spanned the player over the selector.
- Album columns follow the current measured width in one frame callback; removed the 140 ms settled-width timer and its late size jump.
- Artist reads reuse profile results; cached complete profiles avoid another provider check for 24 hours. Library refresh no longer starts artist lookups for every missing portrait. Favorites/analysis changes do not reload the library snapshot. Home reuses listening results until listening/library changes.
- Home has a concise name/count, a resume action, collections, and filled secondary navigation. Native dropped files go to the largest enabled, available music folder. Settings shows that target. Explicit managed-copy commands retain their destination. Re-dropping an existing destination file preserves it and its timestamp.
- Standard mini-player cover is 160 px at 244 × 320, and grows square with available space. Compact/Wide retain their separate dimensions. Resize uses local pointer coordinates, avoiding screen-origin changes. Menus reposition when the expanded native window changes size.
- Playing hearts use analyzed BPM, or a gentle 72 BPM fallback when no analysis is available. Paused, offscreen and Reduced Motion hearts remain still. Existing authored transport morphs remain intact.

## Android companion

Devices puts the paired Mac and copying status first; pairing is a collapsed secondary section. Offline QR scanning uses bundled ZXing, with paste fallback and normal user-initiated camera permission. Notifications open Devices. Filled controls, Figtree, freshly exported Sketch Phone/Download/Downloaded icons (32 × 32 source geometry) and the existing blue palette are retained; artwork colors are optional. Home has shorter shelves, visible navigation actions and offline counts.

Copy jobs use KEEP rather than restart on repeated taps. Connected Wi-Fi/Ethernet is required, including metered home Wi-Fi. Verified songs publish incrementally, without dropping older offline tracks, favorites or collections. New transfers retain full SHA verification and pinned encrypted connections. Existing private copies can reuse a hash receipt only while native nanosecond timestamps, inode, device and size match; invalid stat data falls back to hashing. Android 26 always hashes. No originals or installed model/embedding replacements.

Artwork uses a 32 MiB memory cache and size-bounded SDR decodes (96/256/512/1024 px buckets), shared by shelves and player. Unchanged size buckets reuse decodes. No gain map or HDR brightness restoration. Playing hearts stop when paused or the activity is inactive.

## Measured evidence

Identical WebKit fixture resize sweep, 1200 → 800 px, 20 px steps:

| Measurement | Before | After |
| --- | ---: | ---: |
| Median frame interval | 17 ms | 17 ms |
| 95th percentile | 19 ms | 19 ms |
| Intervals over 25 ms | 0 | 0 |
| Artwork size change after resizing stopped | 40.20 px | 0 px |

This is browser-fixture pacing, not native display FPS. It establishes elimination of the delayed resize jump; it does not establish a higher native frame rate. No speed multiplier is claimed for real Wi-Fi or a physical phone.

## Verification

- UI: 164 unit tests passed.
- Rust: 175 app tests passed; 2 explicit/network tests ignored. Three import tests passed, including duplicate/self-drop preservation.
- WebKit: adaptive navigation 480/720/1280, four scales and Light/Dark artwork/blue/extreme custom; in-window player, menu/Space/queue identity checks; Home theme/scale matrix; mini-player pointer/keyboard resizing, menu expansion; cached artist revisit; gentle heart and Reduced Motion; fixed player identity across breakpoints; artwork square/minimum checks.
- Android unit tests and lint passed. Emulator: paired Devices in Light/Dark, offline receipt tamper/replacement, bounded/reused SDR artwork, player 360 × 480/720, retained duplicate queue entries, Home/Library/Search retention and Vorbis playback passed. Separate approved local Rust TLS fixture copied a verified track/artwork/duplicate collection; playback and a media notification worked after server shutdown.
- Physical Android Wi-Fi, Bluetooth/headset, lock screen/OEM background behavior remain device acceptance gates; no physical phone was connected.

Delivery hash, signing, latest-profile checkpoint and native smoke results are recorded in the release manifest after installation. Installation requires a fresh visible paused/stopped check, preserving the exact current playback and latest profile.
