# Local Find and compact list controls — 2026-10-23

## Result

The current-list toolbar now offers **Find**, with Command-F and Control-F, instead of another global Search action. Sidebar Search and Command-K retain the whole-library command/search panel. Playlist/Sparklist details, Songs, Favorites, Albums, and genre lists get an animated local field. Collection management focuses its existing field, including when the manager is still mounting; artist details retain their uncluttered header.

Find combines words across title/artist/album/genre, normalizes accents and punctuation, tolerates one small spelling error on longer unquoted words, and accepts quoted phrases, field qualifiers, year ranges, favorite/availability predicates, and excluded terms. Results retain original order and duplicate track identities. Filtering never edits saved collections or sends playback commands. Playlist suggestions are hidden while a query is active so unrelated songs cannot be mistaken for matches.

Closing restores the original list scroll and the toolbar's keyboard focus; navigation into details and back retains the query. Command/Control-F reselects the existing query. Open dialogs and menus retain keyboard ownership. Back and Find glyphs are smaller, with stationary hit targets, readable count/clear controls, optional syntax tips, and a 180 ms opacity transition; Reduced Motion removes animation.

## Verification

- 128 UI unit tests passed across 40 files, including smart matching, exclusions, phrases, availability, spelling, and duplicate/order preservation.
- 34 relevant browser cases passed across the fresh runs and corrected test reruns: 13 Find cases plus shared Search, dropdown, Settings, queue, cover, mini-player, and management regressions.
- Geometry checks cover 720×480 and 1280×800, interface scales 80/100/125/150%, Light/Dark artwork colors, compact icons, and no horizontal Find overflow. Existing mini-player checks cover blue/artwork/custom themes and 1×/2× backing transparency.
- Initial Find testing exposed a real lazy-mount focus race in collection management; focus now follows the typed request prop after mount. The corrected case passes.
- An overnight combined run had timeouts and was repeated while preventing idle sleep. All Find cases and other affected cases then passed. The remaining Settings assertion expected zero requests upon returning to Library; the existing automatic model hook intentionally performs one activation refresh. The test now asserts exactly that one refresh, followed by no idle/hidden polling, and passes. No production polling behavior was changed.
- Additional management checks passed for saved collection type separation, local filtering, Smart rule persistence, metadata review/Undo, and the 40-combination window/scale/palette matrix. These older tests were updated to use More → Playlists and explicit Albums navigation, matching the current navigation rather than removed shortcuts. The initial obsolete-navigation run was stopped; corrected cases passed.
- UI production compilation passed. Signed package, runtime hashes, latest-state preservation, and native verification are recorded below after delivery.

This change modifies desktop presentation only: no database migration, playback protocol, OS source, embedded runtime, model pins, or saved sequence changes. Windows/Linux and human animation acceptance remain separate gates.

## Native signed delivery

- Desktop implementation commit `2d12ffd`; primary cherry-pick `f20a400`. OS work was preserved.
- Signed executable SHA256: `c61e2e0baad148824643deb7b903ea72fb97f14c2d5eba02a0c5b3b6833512ea`.
- Unchanged embedded audio executable: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- Unchanged embedded ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
- Optimized native compilation completed in 1m38s; bundle 367.64 MiB. Ad hoc signing and strict deep verification passed. All bundled model files retain their pinned sizes/checksums; no embedded runtime rebuild was run.
- Preserved previous/replaced applications and latest private preferences; SQLite backup passed quick_check. Loaded only after confirming paused playback and clean process exit.
- On Track restored paused at 233 seconds, with 507 songs, 26 favorites, and 12 saved collections. Six saved playlist/collection/Spark tables match the immediate pre-load backup exactly.
- Native Command-F opened Find in Mm. `artist:"MF DOOM"` retained its one matching song; suggested additions hid during filtering. Escape removed Find, restored the suggestions, and returned focus to the Find button. Playback stayed paused and unchanged.
- Native proof: `out/qa/os-foundation/list-find/native-playlist-find.png`. Preserved packages and release manifest: `releases/desktop-list-find-2026-10-23/`.
