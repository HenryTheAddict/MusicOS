# Cover-only review repair — 2026-10-28

The album review previously preselected changed text fields and combined artwork with the same Apply action. The UI also retained a previous album’s generic “songs updated” message after opening another comparison.

Cover art now has an independent **Update cover only** action and a cover-specific confirmation. Its request contains only `artwork`, even if the user has checked album or song fields. Text edits start unchecked and use a separate Review selected details → Apply reviewed details action. The confirmation states the album and number of covered songs; its result explicitly says song and album details are unchanged. Opening another comparison clears that result. Counts use “1 song” correctly.

Original-file publication remains opt-in and retains format/access/backup preflight, paused-playback enforcement, stale comparison checks, and History/Undo. No database, playback, artwork-decoding, Android, runtime, or model changes. A later requested artist-profile correction is included below.

## Investigation

Read-only inspection of the user’s two latest history batches found cover-path changes only: ASTROWORLD (17 songs) and UTOPIA (19 songs). No titles, artists, album names, years, or numbering changed in those batches. No user history was undone or rewritten.

## Verification

- 163 frontend unit tests passed, including checked metadata ignored by cover-only, file-write opt-in, stale message clearing, singular counts, and text-only updates excluding cover art.
- 171 Rust tests passed; one existing ignored test. The new cover-only regression exercises library-only and original-FLAC publication with deliberately different title, artist, album edition, album artist, year, disc, and track proposals. It checks unchanged album identities/membership and tags, other albums untouched, stale rejection, and exact original-file restoration through Undo.
- Three WebKit tests passed: selected field review with Undo, cover-only with checked text fields, and narrow/wide confirmation at 480×480 and 1280×480 across all four interface scales. No horizontal overflow. Shared button/theme styling is retained.
- Production frontend build passed. Signed bundle delivery details and profile/runtime preservation are recorded separately in the release manifest.

Native installation and visual acceptance will be recorded after a fresh paused/stopped playback check. No real music files were edited by these tests.

## Same-name artist correction

The user’s Steve Lacy profile pointed to the jazz saxophonist while their library contains Apollo XXI and Gemini Rights. The existing search returned the saxophonist with score 100 and the modern artist with score 85. Filtering out scores below 95 incorrectly made this look like an unambiguous identity.

Same-name candidates are now considered together. Ambiguous names are checked against exact local album titles and the candidates’ MusicBrainz discographies and artist credits. Discographies page by the number of returned rows, validate totals, and are bounded to 500 entries and eight candidates. Incomplete, conflicting, or unresolved evidence never falls back to the highest score. Previously verified album-context profiles remain usable during outages; an unverified ambiguous profile is no longer published after failed verification. Biography/photo fields are reused only for the same MusicBrainz identity.

The generic implementation contains no Steve Lacy exception. Its live check used an isolated cache and identified `b29020d6-5dbf-4c99-b16f-c009f8a712ff` from both local albums, fetched the correct biography and a portrait credited to Moyopoyo (CC BY-SA 4.0), and preserved source attribution. The production profile cache will be backed up before replacing only Steve Lacy’s two profile entries and adding the verified portrait. Album covers, tracks, tags, saved sequences, and other artist profiles are untouched.

175 Rust tests passed; two explicitly ignored live/environment tests. The requested live profile check also passed when invoked separately. Tests cover the 100-versus-85 ranking case, conflicting identities, missing/truncated/offline results, local artist album selection, and discography pagination. Official identity reference: https://musicbrainz.org/artist/b29020d6-5dbf-4c99-b16f-c009f8a712ff . API behavior: https://musicbrainz.org/doc/MusicBrainz_API#Browse .

## Installed combined delivery

Installed from the isolated desktop branch after a fresh native paused check: Like me at 187.5 seconds. Its saved 187.53734-second position was restored after startup. Native artist-page screenshot showed the correct modern portrait and biography alongside both albums. The installed BAROWEEN review showed 1 song, unchecked metadata fields, the independent Update cover only action, and a cover-specific confirmation. No real cover or metadata Apply action was invoked during acceptance. Window access became unavailable during final cancellation/navigation verification; no playback command was issued.

Signed executable SHA-256: `1ba4b4eb419d6670658155812a186a62cf92f1755789864e93040f83d1ef30a0`. Eight embedded runtime/model files matched the prior app byte for byte; previous app and latest private profile/database were backed up. Release manifest: `/Users/henry/Developer/MusicOS/releases/desktop-cover-profile-2026-10-28/release.json`.

The checkpoint preserved the latest **550 tracks**, two manual playlists and five entries, plus saved collection preferences. During the live check, two additional recorded original-file edits (one artist field and two track numbers) and a favorite change appeared. These newer edits were retained; prior history, track identities, playlists, and entries remained intact. All settings stayed identical except the live playback position. Android was unchanged; its previous signed APK hash remains recorded. OS work remains untouched.
