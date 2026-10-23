# Local Find and compact list controls — 2026-10-23

## Result

The current-list toolbar now offers **Find**, with Command-F and Control-F, instead of another global Search action. Sidebar Search and Command-K retain the whole-library command/search panel. Playlist/Sparklist details, Songs, Favorites, Albums, and genre lists get an animated local field. Collection management focuses its existing field, including when the manager is still mounting; artist details retain their uncluttered header.

Find combines words across title/artist/album/genre, normalizes accents and punctuation, tolerates one small spelling error on longer unquoted words, and accepts quoted phrases, field qualifiers, year ranges, favorite/availability predicates, and excluded terms. Results retain original order and duplicate track identities. Filtering never edits saved collections or sends playback commands. Playlist suggestions are hidden while a query is active so unrelated songs cannot be mistaken for matches.

Closing restores the original list scroll and the toolbar's keyboard focus; navigation into details and back retains the query. Command/Control-F reselects the existing query. Open dialogs and menus retain keyboard ownership. Back and Find glyphs are smaller, with stationary hit targets, readable count/clear controls, optional syntax tips, and a 180 ms opacity transition; Reduced Motion removes animation.

## Verification

- 128 UI unit tests passed across 40 files, including smart matching, exclusions, phrases, availability, spelling, and duplicate/order preservation.
- 31 relevant browser cases passed across the fresh run and one corrected test rerun: 13 Find cases plus shared Search, dropdown, Settings, queue, cover, and mini-player regressions.
- Geometry checks cover 720×480 and 1280×800, interface scales 80/100/125/150%, Light/Dark artwork colors, compact icons, and no horizontal Find overflow. Existing mini-player checks cover blue/artwork/custom themes and 1×/2× backing transparency.
- Initial Find testing exposed a real lazy-mount focus race in collection management; focus now follows the typed request prop after mount. The corrected case passes.
- An overnight combined run had timeouts and was repeated while preventing idle sleep. All Find cases and other affected cases then passed. The remaining Settings assertion expected zero requests upon returning to Library; the existing automatic model hook intentionally performs one activation refresh. The test now asserts exactly that one refresh, followed by no idle/hidden polling, and passes. No production polling behavior was changed.
- UI production compilation passed. Signed package, runtime hashes, latest-state preservation, and native verification are recorded below after delivery.

This change modifies desktop presentation only: no database migration, playback protocol, OS source, embedded runtime, model pins, or saved sequence changes. Windows/Linux and human animation acceptance remain separate gates.
