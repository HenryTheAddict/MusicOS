# Metadata and artwork

[Guide home](README.md)

Open **Settings → Library → Metadata & artwork**. Source tags and music files stay intact; edits are library overrides.

## Find → Review → Apply

Indexing prepares suggestions in the existing background worker. **Find metadata & artwork** explicitly checks again, including matches you previously kept. MusicBrainz and Cover Art Archive suggestions do not automatically change song fields or canonical artwork.

Search Suggestions, then open a song to compare current and proposed values. Only changed metadata fields are preselected. Artwork requires its own selection; click a preview to inspect it larger. Review source links and the edition carefully: a match score is similarity, not verification that a live, Deluxe, or other release is your edition.

Choose **Apply selected changes** or **Keep current**. Kept matches do not return in routine unchanged-file lookups. If the song or suggestion changes while you review, MusicOS refuses the stale application and asks for a refreshed comparison.

## Edit songs

Search and select songs; filtering retains your selection. Each field offers Leave unchanged or Set value. Genre, year, track number, and disc number also offer Clear value. Title edits require exactly one song; titles, artist names, and album names cannot be cleared.

Shared values appear beside each field, with Multiple values for mixed selections. Review the summary before applying a bulk edit.

## History and Undo

History displays the latest 50 change batches across restarts. Undo restores the previous values, override protection, and relevant artwork references transactionally. Earlier records remain stored; this screen displays the latest 50.

Later conflicting edits or changed source identities prevent automatic Undo. Expand Compare changes to inspect before, after, and current values. No partially undone bulk batch is committed.

## Artist profiles

Artist profiles has its own cached/checking/unavailable status and retry action. Cached portraits and biographies remain available offline and during provider failures. Technical details stay behind Details rather than appearing in collection navigation.
