# Importing and rescanning

[Guide home](README.md)

## Add music

Open **Settings → Library → Music folders**. **Add a music folder** references files in place. **Copy into MusicOS** imports copies into managed storage. A referenced drive must remain connected for playback.

Removing a referenced folder hides its songs while retaining files, metadata edits, favorites, and collection membership. Restore the folder from the same page.

## Choose the right scan

| Action | Location | Purpose |
| --- | --- | --- |
| Rescan folders | Settings → Library → Music folders | Discover additions and filesystem changes |
| Rescan audio | Settings → Audio → Quality & analysis | Refresh missing, failed, changed, or outdated specifications and musical analysis |
| Recheck every available song | Beside Rescan audio | Explicitly recompute specifications and musical analysis even when current results exist |
| Find metadata & artwork | Settings → Library → Metadata & artwork | Prepare suggestions; review every field and artwork change before applying |

Rescanning audio uses a background worker. Upcoming tracks retain priority. Progress, Pause, Resume, and Cancel appear while the worker runs. Completed results remain cached if canceled. Unavailable files are skipped; reconnect their folder and rescan it first.

A failed audio refresh retains a previously valid result for the unchanged file. This does not claim the failed refresh succeeded. Expand the failure details, correct the problem, and retry. Scans preserve source files and user metadata overrides.

## Albums and credits

Album grouping uses album artist or the primary contributor, while explicit editions and verified distinct releases remain separate. Guest credits belong to individual songs. Multi-disc albums retain disc sections and disc-local numbering. Canonical artwork gives each album one stable cover and adaptive palette.

See [Metadata and artwork](metadata.md) for comparisons, editing, and Undo.

If an older import has no track number, MusicOS can recover a filename prefix such as `12 ARE WE STILL FRIENDS-.flac` for album display and playback ordering. Explicit user track-number overrides stay protected. Tracks with no usable number sort after numbered tracks rather than becoming the first song; files and source tags are not rewritten.
