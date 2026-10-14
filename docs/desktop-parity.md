# Native Control Center parity gate

The cutover is complete. The legacy Electron application, Vite, React, Tailwind, and
superseded frontend tests have been removed. The native GPUI desktop application is the
sole Control Center.

| Area | Native owner | Current gate |
| --- | --- | --- |
| Library paging and search | `LocalLibraryRepository`, GPUI virtualized table | Direct SQLite snapshot plus indexed virtualized rows; 10,000-row GPUI test |
| Albums, artists, songs, favorites, Flow | GPUI library views | Implemented |
| Playback controls | `LocalPlaybackService` and toolbar | Dedicated local audio thread, persistent queue/position/preferences, metadata, artwork, timeline, and controls implemented without a companion |
| Background activity | `operations`, GPUI Activity Center | Typed import, scan, sync, and emulator tasks with determinate/indeterminate progress, cancellation, failure, bounded history, and animated aggregate state implemented |
| Managed import | `library_import` and GPUI path dialog | Implemented with atomic verified copies and cancellation |
| Reference folders | `reference_sources` | Add, watch, offline/reconnect, pause, changed/full rescan, reveal, scan-on-launch preference, and confirmed removal implemented |
| Metadata and artwork | `local::enrichment`, SQLite provenance/cache | Whole-library MusicBrainz matching, confidence review, user-edit protection, validated Cover Art Archive downloads, and bounded disk cache implemented |
| Folder sync | `sync`, `transcode` | Implemented with boundaries, checksums, retries, cancellation, progress, history, and adjustable OGG Vorbis profiles that preserve local originals |
| Playlist reconciliation | `playlist_sync` | Native three-way plan/conflicts implemented |
| Device Preview lifecycle and logs | `runtime_manager` | Optional launch/stop, silent audio backend, controls window, F1, and preview-only show-controls endpoint implemented |
| Diagnostics and destructive reset | `diagnostics` | Export, artwork cache, repair, reveal, and recoverable confirmed reset implemented |
| Settings and migration | `storage` | Categorized library, metadata, playback, format, appearance, preview, and advanced settings plus versioned config and idempotent Electron-data import implemented |
| Updates, backup, restore | Device HTTP API and local repository backup | Remote update install plus standalone local playlist backup and restore implemented |
| Automation CLI | Rust `musicos-agent` | Command shape, JSON modes, and exit codes implemented |
| Packaging | `cargo-packager` 0.11.8 | Configured; unsigned platform artifacts |
| Embedded UI | framebuffer renderer | Classic-iPod visual system and time-based Visuals/Balanced/Battery Saver/Reduced motion profiles implemented; flow tests pass |
| Native shell | GPUI platform integration | Application menus, playback/view shortcuts, transparent macOS titlebar, and system light/dark tokens implemented |
| Typography | bundled fonts | Existing open Noto/Raster fallback active |
| Accessibility | GPUI controls | Keyboard search/navigation, visible labels, and reduced-motion modes implemented; GPUI 0.2.2 screen-reader semantic-tree limitation is documented |
