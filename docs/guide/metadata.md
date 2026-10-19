# Metadata and artwork

[Guide home](README.md)

Open **Settings → Library → Metadata & artwork**. Changes default to MusicOS-only overrides. Writing to music files is a separate choice for each application.

## Find → Review → Apply

Indexing prepares suggestions in the existing background worker. **Find metadata & artwork** explicitly checks again, including matches you previously kept. MusicBrainz and Cover Art Archive suggestions do not automatically change song fields or canonical artwork.

Search Suggestions by album or artist, then open an album to compare shared fields and its recordings. Album Artist, album and year appear above the track comparison. Only changed metadata fields are preselected. Artwork requires its own selection and an album-wide confirmation showing the affected song count. Current and proposed resolution and source appear beside the images. Editions stay separate. Review source links and the edition carefully: a match score is similarity, not verification that a live, Deluxe, or other release is your edition.

Choose **Apply selected changes** or **Keep current**. Kept matches do not return in routine unchanged-file lookups. If the song or suggestion changes while you review, MusicOS refuses the stale application and asks for a refreshed comparison.

## Edit songs

Open Edit metadata from an album or song menu to start with that selection. Browse Songs or Albums; filtering retains your selection. Each field offers Leave unchanged or Set value. Album Artist, genre, year, track number, and disc number also offer Clear value. Title edits require exactly one song; titles, contributor names, and album names cannot be cleared. Album Artist changes leave individual contributor credits intact.

Shared values appear beside each field, with Multiple values for mixed selections. Review the summary before applying a bulk edit.

## History and Undo

History displays the latest 50 change batches across restarts. Undo restores the previous values, override protection, and relevant artwork references transactionally. Earlier records remain stored; this screen displays the latest 50.

Later conflicting edits or changed source identities prevent automatic Undo. Expand Compare changes to inspect before, after, and current values. No partially undone bulk batch is committed.

## Artist profiles

Artist profiles has its own cached/checking/unavailable status and retry action. Cached portraits and biographies remain available offline and during provider failures. Technical details stay behind Details rather than appearing in collection navigation.

## Ignore one match

Choose **Ignore suggestion** beside a suggestion or in its comparison. This hides that candidate across restarts and routine rescans, without changing any song fields or artwork. A genuinely different recording, release, or suggested value may still be offered.

The **Ignored** tab lists hidden matches. **Restore suggestion** allows the current matching candidate to be reviewed again. Changed songs or candidates still require a fresh comparison before applying.

## Also write to music files

This option starts off for every application. Review the affected files, supported fields, permissions and required backup space before enabling it. Pause playback before replacing files. MusicOS stages and validates format-aware tag changes without transcoding the audio. Unsupported or protected files fail with a specific error; a failed stage leaves originals untouched.

Original-file backups remain stored until you explicitly remove them under **History → Original-file backups**. File Undo checks both current metadata and file identities before restoring anything. Later changes stop automatic restoration rather than being overwritten. Removing a backup disables file Undo for that application.

Interrupted file work uses a recovery journal. MusicOS checks it at startup; **Retry interrupted file work** checks again after you resolve the reported access or identity conflict. Do not remove or edit journal files to force an operation through.
