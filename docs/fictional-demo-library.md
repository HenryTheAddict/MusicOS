# Fictional development library

`/?fixture=1` uses imaginary artists and albums, original placeholder lyrics,
and locally generated abstract artwork. It does not represent a real artist's
catalogue. The iOS browser fixture uses the same fictional artist names.

Run `node scripts/generate-demo-artwork.mjs` to regenerate the album, artist,
legacy preview and playlist-picker artwork, including playlist previews.
This replaces the former network cover scraper; no external image service is
used. Unused stock covers and the checked-in real-artwork cache were removed.

Fixture identities and contracts remain unchanged: 144 tracks, 12 albums,
seven artists, stable IDs, artist grouping, durations and playback behavior.
The existing original lyric lines retain their timing, word boundaries,
instrumental-break variants and error cases so lyrics UI work can proceed
independently. The former real song titles in album 6 are now original titles.

Wide Home, Albums and Now Playing/lyrics captures were taken before and after
the replacement. The README images are generated from the updated browser
fixtures, not a user's music library.

Historical QA documents describe past runs against real personal libraries;
those records have not been relabeled as fictional verification results.
Explicit live-provider contract tests and artist-alias handling also retain
the real identities they must query or recognize. Neither is demo content.

PR #48 changes lyric presentation, not fixture contracts. Its original lyric
text and fixture modes are retained here. The open README screenshot PR #46
contains separate real-library captures and should use the fictional captures
when rebased; they are not present on this branch's base. The Now Playing
polish PR #31 is already merged on this base.

## Verification

- Desktop TypeScript typecheck passed.
- Vitest: 82 files, 289 tests passed.
- Nine targeted Playwright specs exercised fixture navigation, artists,
  artwork colors, Now Playing/mini-player, lyric timing, breaks and errors.
  63 tests passed across the initial run and its corrected/environment-fixed
  recheck. Three unchanged `lyrics-composition.spec.ts` tests fail identically
  on a separate pristine `main` worktree: the narrow reading bounds differ by
  2px, the old blur assertion expects `blur(0.6px)` instead of `none`, and the
  old word-feather assertion expects `42.5%` instead of `38%`. They were left
  unchanged to avoid overlapping PR #48's lyric-presentation work.
- Rust enrichment tests: 11 passed. Targeted Tauri test execution was blocked
  by the absent `binaries/musicos-mac-x86_64-unknown-linux-gnu` sidecar; no
  build configuration or placeholder binary was added to bypass this.
- Repository search found no former real artist identities in the desktop
  or mobile browser fixtures, UI tests, or README. Historical QA records,
  explicit network contracts and actual artist-alias rules are intentionally
  distinguished from demo data as described above.
