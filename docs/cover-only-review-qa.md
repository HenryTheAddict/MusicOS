# Cover-only review repair — 2026-10-28

The album review previously preselected changed text fields and combined artwork with the same Apply action. The UI also retained a previous album’s generic “songs updated” message after opening another comparison.

Cover art now has an independent **Update cover only** action and a cover-specific confirmation. Its request contains only `artwork`, even if the user has checked album or song fields. Text edits start unchecked and use a separate Review selected details → Apply reviewed details action. The confirmation states the album and number of covered songs; its result explicitly says song and album details are unchanged. Opening another comparison clears that result. Counts use “1 song” correctly.

Original-file publication remains opt-in and retains format/access/backup preflight, paused-playback enforcement, stale comparison checks, and History/Undo. No database, playback, artwork-decoding, Android, runtime, or model changes.

## Investigation

Read-only inspection of the user’s two latest history batches found cover-path changes only: ASTROWORLD (17 songs) and UTOPIA (19 songs). No titles, artists, album names, years, or numbering changed in those batches. No user history was undone or rewritten.

## Verification

- 163 frontend unit tests passed, including checked metadata ignored by cover-only, file-write opt-in, stale message clearing, singular counts, and text-only updates excluding cover art.
- 171 Rust tests passed; one existing ignored test. The new cover-only regression exercises library-only and original-FLAC publication with deliberately different title, artist, album edition, album artist, year, disc, and track proposals. It checks unchanged album identities/membership and tags, other albums untouched, stale rejection, and exact original-file restoration through Undo.
- Three WebKit tests passed: selected field review with Undo, cover-only with checked text fields, and narrow/wide confirmation at 480×480 and 1280×480 across all four interface scales. No horizontal overflow. Shared button/theme styling is retained.
- Production frontend build passed. Signed bundle delivery details and profile/runtime preservation are recorded separately in the release manifest.

Native installation and visual acceptance will be recorded after a fresh paused/stopped playback check. No real music files were edited by these tests.
