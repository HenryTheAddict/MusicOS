# Artist credits, Album Artists, and managed-copy polish

October 15, 2026. Current layouts, supplied icons, stored collections and source files retained.

## Changes

- Spaced plus credits split into separate contributors in backend grouping and song rows. Literal plus names such as C++ are retained. Two displayed contributors use & without claiming a featured role; names stay clickable. Joint album-artist tags expose individual identities too.
- Persisted Album Artists switch lives opposite the Artists heading. It filters to album-level credits, including joint credits and verified identity spellings. Guest-only identities remain in metadata and track membership, hidden only from this grid mode.
- Native dropped files/folders copy into managed storage automatically, as before. A new external-file-only overlay explains the action; internal queue drags do not trigger it. Managed import destinations now prefer embedded album artist and album tags over source-folder names, so loose files are organized correctly. Sources remain untouched; checksum verification, collisions, cancellation and the managed import index are retained.
- Metadata suggestions gain artwork thumbnails and clearer grouped review actions. Bulk inputs use theme surfaces instead of hard-coded white. File organization is separated from import/rescan controls with concise copy and a finite 160 ms sheet entrance; reduced motion removes it. No ornamental gradients were added.
- Corrected selected Settings hover colors: the old hover background overrode accent selection while retaining dark accent ink in dark themes.
- Folder-access recovery points to reselection through the native picker and macOS Files and Folders controls. No administrator privilege request or broad security change is introduced.

## Validation

- 70 frontend tests, 114 app tests, 79 desktop-core tests passed.
- A real temporary WAV with album-artist and album tags proves managed destination hints work without Artist/Album source folders and do not rewrite bytes.
- Browser management checks passed in 40 compact/wide × scale × palette cases. Metadata application/Undo, ignoring, playlist rules and context-menu checks passed. Compact dark screenshot inspected separately and selected hover contrast corrected.
- Album Artists opposite-edge placement and restart persistence passed at 720×480 and 1280×800. Unit tests verify guest-only filtering and joint album artists. The first persistence fixture reloaded before the asynchronous write completed; final test waits for persisted state.
- Four queue/menu/drag checks passed, including internal-file-drop isolation and virtualized queue autoscroll/cancellation.
- No user music was copied or moved during implementation tests. Native package checks below do not apply metadata or organize files.
- Windows/Linux, 120 Hz stress and human audio listening acceptance remain separate unverified gates.

## Delivered package

- Signed local Apple Silicon build: `target/release/bundle/macos/MusicOS.app`, 98.40 MiB.
- Executable SHA-256: `a58fea7bb168f026375060b4b3ca6d620ceb8d078fa754dcafa4d6692d3b7c69`.
- Ad-hoc deep/strict codesign verification passed. Embedded sidecar and ONNX runtime hashes remain unchanged.
- Loaded while paused; source position restored to 8.1 seconds. Native state retained 477 songs, 11 favorites and saved collections.
- Native Artists view shows Album Artists enabled, with guest-only entries such as 21 Savage, Anitta and remix guests hidden; PinkPantheress and album-level artists remain. User began interacting with the new toggle, so the selection was preserved and further UI actions stopped.
- No real user files were copied/moved as part of this delivery. Existing automatic copy pipeline is exercised by temporary fixtures; native external Finder-drop acceptance remains distinct from internal queue-drag coverage.
- Logs: `/tmp/musicos-artists-package.log`, `/tmp/musicos-artists-app-tests.log`, `/tmp/musicos-artists-core-tests.log`, `/tmp/musicos-artists-ui.log`, `/tmp/musicos-artists-import-tests.log`, `/tmp/musicos-artists-browser.log`, `/tmp/musicos-artists-final-browser.log`, `/tmp/musicos-artists-toolbar-final.log`.
