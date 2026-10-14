# React / Tauri desktop

`ui-desktop/src` owns semantic HTML, responsive layouts, bundled Figtree font weights,
accessible Radix menus, route history, static theme gradients, and virtualized
large song lists, collection grids, and playback queues. `src-tauri` owns the
command bridge. `crates/musicos-desktop` supplies reusable services with default
features disabled; GPUI is not linked into the new app.

## Data and preferences

The existing Rust desktop config determines SQLite and managed media locations.
No replacement demo database is created in production. `MUSICOS_APP_DATA` can
isolate a development/test profile. Existing `MUSICOS_MEDIA_DB`,
`MUSICOS_MEDIA_DIR`, `MUSICOS_SYNC_TARGET`, and `MUSICOS_COMPANION_PORT` overrides
remain supported by those services. Existing additive database migrations and
pre-migration backups are reused.

`interface-v1.json` in the Rust desktop application data directory stores the
three appearance preferences. Writes use the existing atomic-write service.
Invalid fields fall back independently to Henry / 100% / colors off. The first
launch imports the compatible SDL `MusicOS/MusicOSDesktop/preferences.txt` when
no new preference file exists. Errors appear inline; changes remain active in
that session if writing fails.

Library identities use database track IDs and namespaced, length-delimited hashes
for album/artist/genre groups. Counts and durations come from indexed records.
Missing files remain visible with disabled playback. Reference sources preserve
original files; copied imports use existing checksum-verified managed imports.
Folder imports, rescans, enrichment, and transfers share one cancellable task
slot. A rescan is explicit in Settings > Library > Music folders.

## Artwork and providers

Embedded/folder artwork is extracted lazily through Rust for visible album cards
and detail pages. File access is granted only to artwork paths resolved from the
library or provider cache. Palette results cache on path and modification time;
static backgrounds crossfade once when the source changes. Album art takes
precedence on album details; artist portrait then first album on artist details;
playing-track art elsewhere. Missing/unreadable art returns blue. Interface size
uses rem-based layout units and leaves device pixel density to the WebView.

Discover has been removed from the desktop UI; its provider services remain
available internally. MusicBrainz/CAA enrichment uses
existing matching and manual-overlay precedence; uncertain matches appear in
Settings > Library > Metadata & artwork. Artist biography lookup requires an unambiguous
exact MusicBrainz match and follows verified Wikipedia/Wikimedia links, with
source attribution and unavailable states. Portrait downloads follow the same
verified Wikidata identity and expose Wikimedia photo credits and source links.

Playback commands share the existing Rust audio worker. Events publish progress
and state at 250ms while queue membership is sent only when its revision changes.
Tauri alone enables Symphonia decoding plus the `desktop-seek-length` fix in
`third_party/rodio` so FLAC seeking works. The patch preserves the source position
while exposing its length; its feature is off in the separately built embedded
runtime. The packaged app includes the unchanged `musicos-mac` preview sidecar. Devices
uses existing status, sync planning, upload, format, and cancellation services.

## Build and launch

`./run.sh` starts Tauri development. `npm run build` prepares the UI and native
preview before building the app. `npm run package -- --bundles app` produces a
macOS bundle under `target/release/bundle/macos`. Tauri also supports its normal
platform bundle targets. The app uses native macOS overlay titlebar controls;
Windows/Linux use platform decorations. Cross-platform code is present, but
native acceptance on each OS must be recorded separately.

Development-only `?fixture=1` enables deterministic UI fixtures, including
`&large=1` for 10,000 songs. Production builds remove that import branch. These
fixtures are test evidence for frontend behavior, not evidence of native audio,
window controls, or device hardware.

## Manual metadata and navigation

Settings > Library > Metadata & artwork > Edit metadata supports a filtered,
virtualized multi-selection and explicit field toggles. Artist, album, genre,
year, track and disc numbers can be edited in batches; titles require one song.
Only checked fields are written. The Rust command validates all selected IDs
before committing one existing database override transaction. Edits receive the
`user` origin, protecting them from enrichment and rescan; source file tags are
preserved. A track overflow menu opens the editor with that track selected.

Library categories always open their remembered browsing locations. A separate
Back control follows route history, including search, scroll and selection.
Refreshing metadata prunes obsolete collection IDs from the history. The logo
sits beside native traffic lights, reducing to its icon in compact windows.
Search and every song view use the resolved artwork palette. Search focus has a
finite highlight sweep and lens transition; reduced motion disables both.
