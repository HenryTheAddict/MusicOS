# Selector close, current Sketch icons and missing-cover recovery — 2026-10-25

The section selector keeps one painted header surface in its closed, opening,
open and closing states. Previously, final unmount replaced a transparent
animated header with the ordinary button's 140 ms background transition. The
new permanent outline removes that fill handoff and avoids changing the header's
stacking context at completion. The existing 140 ms reveal, opaque portaled
choices, shadow, chevron, focus and deferred navigation remain.

The app embeds exported vectors; it does not read the open Sketch file at runtime.
`/Users/henry/Pictures/Sketchy/musicosdesk.sketch`, saved at 20:26, was read and
exported with Sketch's own exporter. Updated Sync, tiny Sync, Album, Lossless,
Smart Playlist and Depth geometry is refreshed. Sixteen new visible groups are
included, including Metadata, Settings, Finder/folder, Lyrics, Cloud and Drive
families. Existing Finder and metadata actions use their dedicated icons; folder
relocation and section Settings use the new supplied icons. The duplicate tiny
Sync arrows master is selected instead of the dots version. The new Edit master
contains only its white export canvas, so it is recorded as empty and not used.
Only full-frame white export canvases were omitted. Authored transport masters,
other unchanged vectors and the source Sketch document are preserved.

The user's screenshot also showed missing covers. A known null canonical album
cover previously suppressed both the recovered result and the recovery request
when a stale track source still existed in the response. Visible covers now use
the existing local recovery command in that case and accept its result. Valid
canonical covers keep precedence; portraits keep their separate path. Recovery
stays visibility-triggered and does not reprocess the library or write audio tags.

## Verification

Production UI compilation and all 157 unit tests pass. Coverage includes missing
canonical art with a stale source, valid selected-cover precedence, no-artwork
results, portrait isolation and transparent, nonempty exported vectors retaining
their paths through theme adaptation.

Six focused Chromium cases and four WebKit cases pass. Checks include the last
110/130/139 ms of retraction, unchanged closed paint after unmount and subsequent
frames, no competing button transition, stable stacking, no remaining menu work,
opaque pixels over an album, late palette updates, reversal, deferred navigation,
Escape, focus restoration and Reduced Motion. Native installation and latest
profile preservation are recorded below when complete.

## Native recovery follow-up

The first installed candidate (`36ad8ad`, primary `83a1b08`) verified the refreshed
Sync in Settings and the selector surface. Native checking exposed another layer
of missing-cover recovery: legacy extraction rebuilt local art, then returned the
now-deleted user override path from the database accessor. The override remains
intact. Desktop recovery now accepts only a readable image, falls back to the
same recording's embedded cover when needed, and publishes that fallback through
the existing canonical artwork cache. No schema, tag or collection changes are
introduced. A native regression passes with a deleted explicit override and an
embedded-cover FLAC: the result is readable, the override is unchanged, the next
canonical lookup uses the recovered image, and audio bytes are identical.


## Delivery — 2026-10-26

The final desktop source is `f135e0f`, mirrored to primary as `98415e2`.
The locally signed build is installed. Executable SHA-256:
`cc0c82877c9060e7368b554f03bf021ed095340304c50dfe8bbd83160959faa9`.
Strict signature verification passes. All eight embedded runtime/model hashes
match the prior app; OS sources are untouched. Release, previous app and private
latest-profile checkpoint are retained in
`releases/desktop-icons-flicker-2026-10-26-final/`.

A fresh paused check on Stateside at 2:14 preceded each clean quit/install.
The final app launched paused; the user then resumed playback. Playback was left
running. Six collection tables, 34 favorites, 508 tracks, interface preferences,
integration configuration and persisted playback fields match the immediate
checkpoint; persisted position delta is zero (134.166 seconds). Live playback
progress after the user resumed is intentionally independent of that checkpoint.

Native screenshots from the preceding candidate verify updated Sync on the
Integrations category and Reset theme control, dedicated Settings/device icons,
and the stable closed selector surface. These assets/CSS are identical in the
final package. Final real-library cover recovery remains unverified: the user
was interacting with the window, and the subsequent read-only computer-use
capture timed out. No further UI actions were attempted. The native recovery
regression passes; Junk has no embedded or sibling image available locally and
may retain its placeholder. Human motion acceptance remains separate.

Logs and screenshots are retained in `out/qa/os-foundation/icons-flicker/`.
The source Sketch document remains at its original modification time and size.
