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
