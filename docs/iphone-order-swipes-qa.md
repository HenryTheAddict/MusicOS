# iPhone library ordering and swipes — 2026-11-07

The downloaded catalogue was displayed in transfer order. Album details inherited that order, so songs copied alphabetically did not play in disc/track order. The phone now sorts album groups naturally by title, album songs by disc/track number, and the song catalogue by title. Artist details group songs by album and disc/track. Unknown track numbers follow numbered songs on their disc. Stable album IDs keep duplicate editions separate. Presentation sorts copy their inputs; saved playlists, Sparklist sequences and queue entries retain their exact order and duplicates.

Horizontal swipes now have one native touch listener owner. Axis locking permits ordinary vertical list scrolling, while song rows follow touch movement and reveal Play next. The mini-player supports previous/next and upward expansion; pulling the player header down dismisses it. A leading-edge Back gesture also works over detail song rows. Controls, text inputs, category scrolling and reorder handles keep their own gestures. Cancellation/unmount/resize removes temporary transforms and pending frames. Keyboard and visible buttons remain available.

Reordering computes the final destination from the actual release position, including scroll offset, rather than the previous animation frame. This fixes quick releases that could leave an item in the previous slot.

## Verification

- Phone TypeScript checks and production build passed.
- WebKit tests use a mobile viewport and touch capability. Dedicated tests dispatch touch-shaped events to native listeners for axis lock, row reveal/queue insertion, mini-player skips/expansion, dismissal and edge Back, with no accidental playback. These synthetic events establish action routing, not physical iPhone gesture feel.
- Browser tests also cover natural album order, multi-disc song/playback order from reversed transfer fixtures, alphabetical catalogue display, exact saved duplicate playlist order, queue editing/reordering, drafts and narrow/landscape fitting.
- Native simulator harness: all 32 checks passed, including supported offline formats, queue identity, playlist revisions, playback and background transfer checks. No native audio service changes are part of this repair.
- Physical iPhone gesture acceptance remains pending. Signed artifact and before/after profile preservation are recorded separately in the delivery checkpoint when installed.

The previous iPhone build/profile are preserved. Desktop, OS work, original media, Wi-Fi manifests and protected runtime/model bundles remain unchanged.
