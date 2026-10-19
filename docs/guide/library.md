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

## Removed and offline folders

**Remove from library** hides a folder; it never deletes files or song records. Open **Removed folders** on this page to see its path and retained song count, then choose **Restore folder**. Favorites, metadata overrides, and playlist membership remain attached to the same songs.

An enabled folder on a disconnected drive remains visible as **Offline**. Reconnect it and rescan rather than adding another copy.

Use **Relocate** after moving a referenced folder. MusicOS compares relative paths, file sizes, and content hashes before changing locations. Unmatched songs retain their previous paths for recovery. The managed MusicOS folder cannot be removed or relocated here.

## Smartly organize my files

In **Settings → Library → Music folders**, choose **Smartly organize my files** to review proposed moves. It uses the library’s album artist (or primary artist) and album name to place originals in `Artist/Album` folders inside their existing music folder. Filenames and embedded tags stay unchanged. Nothing moves until you choose **Move files** in the preview. Pause playback first.

Existing destination files are never replaced. Filename collisions use a stable song-ID suffix. Files in managed storage, unavailable files, and symbolic-link files are left in place. Empty folders and companion files such as PDFs stay where they are. External playlists outside MusicOS retain their own paths and are not rewritten. The move uses same-volume hard links before removing originals; a filesystem that does not support those links leaves originals untouched and reports the failure.

**Undo organization** restores the recorded original paths, including after restart. It stops if a recording changed or its original location contains different bytes. MusicOS keeps track IDs, favorites, metadata overrides, collection membership, and duplicate queue entries; queued paths refresh after moving. Moves are recorded individually, so if a later file fails, already completed moves remain usable and undoable.

## Singles outside dedicated folders

Release-type tags, including MusicBrainz/ID3 and MP4 freeform variants, identify Singles and EPs independently of folder names. An explicit album-tag suffix such as `- Single` or `(EP)` is recognized too. A tagged single with a title but no album tag gets its own release title instead of sharing the enclosing loose-files folder with other singles; manual album overrides take precedence. Updated tag caches refresh incrementally.

Enable **Separate Singles & EPs** in the Albums filter to show the separate section. Explicit release metadata wins. Otherwise 1–3 songs are shown as a Single, and 4–6 songs under 30 minutes as an EP. Tagged release totals take precedence over the number imported, so a partial album stays an album. Untagged loose songs receive individual release identities instead of disappearing into Unknown Album.

## Dropping music and managing access

Drag audio files or folders into MusicOS to copy them into managed storage. The drop overlay says **Copy music into MusicOS**; originals stay where they are. Embedded album artist and album tags choose `Artist/Album` destinations even when the source files are loose. The existing hierarchy is the fallback for missing tags. Copies use checksum verification and the managed import index to avoid importing the same bytes repeatedly. Internal queue dragging does not trigger the file-copy overlay.

File organization runs with your normal macOS account permissions. It does not request blanket administrator access. If a protected folder denies access, select it again through **Relocate** or the folder picker and check MusicOS in **System Settings → Privacy & Security → Files and Folders**. Original files remain intact on failed copies or failed moves.

## Album Artists

The switch on the right of the **Artists** bar shows artists credited at album level and hides guest-only contributors. It does not remove their credits or tracks. Joint album artists remain visible, and the choice persists after restart. Spaced ` + ` credits are separated into clickable names; literal plus signs inside a name stay intact.

## Forget removed folders

**Forget library records…** is separate from removing a folder. Its confirmation previews affected songs, favorites, metadata and collection memberships. It removes only records owned by that disabled source; original files and other sources remain intact. Remove affected entries from Up next first. The configured managed library stays protected; empty legacy managed-source entries can be cleaned up.

Organization previews check access, destination collisions and filesystem support before moving anything. Progress and **Stop after this file** operate between files. Completed moves remain usable and undoable if a later file fails. Interrupted work is journaled and checked on restart; use the retry action after resolving a reported access conflict.
