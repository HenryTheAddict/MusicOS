# MusicOS playlists, motion, and DJmode — 2026-10-09

Implemented manual playlist Library destinations, presentation fields, managed artwork,
ordered M3U import/export, and editing that preserves hidden-folder membership.
Legacy rule collections retain their storage identities under Rule playlists.
Collection edits leave the current playback queue intact.

The task banner and dismiss control follow the active palette. Favorites use the
original two Sketch heart SVGs with separate fill/stroke colors, a finite pop and
sparks, reduced-motion handling, serialized saves, and rollback on failure.
Selected songs have no left stripe. Queue items support drag and keyboard reordering,
removal, and clearing upcoming songs. Volume is shared between Queue and Audio Settings,
with mute/restore, a themed rail, percentage readout, and coalesced Rust writes.

Artist cards use verified portraits or the existing Sketch artist placeholder;
album covers are no longer displayed as portraits. Artist selection has no pale
card panel behind its image/caption. Count labels handle singulars. DJmode and
AudioBot were exported from the open Sketch document without modifying it; only
the white export canvas was omitted. Original vectors and gradient opacity remain.

Desktop-only Rust analysis samples beginning/middle/later excerpts and stores
versioned BPM/key estimates against file size, modification time and path. It
supports pause/cancel and one worker. Unknown keys remain unknown. DJmode orders
upcoming songs using tempo/key compatibility and artist diversity, including when
starting a new sequence with DJmode already enabled. It performs four-second
crossfades through two real decoder sinks. No beat matching or time stretching.

Verification evidence:
- 31 Rust app tests passed before the additional DJ ordering test; that new test
  passed separately. Desktop service checks: 46 passed, including real decoder
  crossfade/seek/pause at zero volume on macOS.
- 22 frontend unit checks passed, including rapid volume changes and write failure.
- Earlier full browser suite: 40 passed. Added playlist/queue layout check passed
  at 720x480 and 1280x800 across 80/100/125/150%. Latest targeted run: four passed,
  covering heart rapid/reduced motion, playlists, layouts, artist cards and volume.
- Browser captures: `out/rebuild-qa/artist-grid-no-panel-1280.png` and
  `out/rebuild-qa/queue-volume-icons-720.png`.
- An isolated packaged macOS app analyzed three generated local WAV files while
  playing. Native SQLite cache contains real BPM/key estimates for all three.
  This does not establish real-music tempo/key accuracy or transition listening quality.

Audio Sparklist generation remains disabled. CLAP exported and ran in research
prototypes, but listening comparisons, production model delivery/download controls,
Rust preprocessing, packaged inference and offline generation are pending.
No metadata-filter mix is presented as audio analysis. Windows/Linux remain unverified.
The user's active playing Verification app and its data were left intact.

Latest native pass: the refreshed, signed Motion QA app displayed both custom icons,
restored muted volume to 50%, then muted back to 0%. Enabling DJmode reordered the
three-track queue from First light / Late night / Open sky to First light / Open sky /
Late night using cached estimates. Capture: `out/rebuild-qa/native-volume-dj-icons.png`.
The isolated test app was closed afterward. The signed release is
`target/release/bundle/macos/MusicOS.app`; the user's playing app was not restarted.
