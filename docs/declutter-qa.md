# Mini-player settings and artist portrait declutter — 2026-10-25

The size picker now presents a compact Standard / Compact / Wide selector and
one inert preview of the selected size. Dimensions and the size description are
shown once. Artwork / Up next remains a separate preview choice. The preview
reuses the actual mini-player presentation and its existing density metrics;
window sizes, playback commands, metadata requests and preferences are unchanged.

Artist photos no longer produce floating genre, year, album or song badges on
hover or focus. The existing library statistics remain together below the photo.
Photo attribution is a separate, initially collapsed Photo credit disclosure
below the playback controls, retaining photographer, license and source action.
The photo still opens the artwork viewer and regains focus after Escape. The
full biography and its expansion, colorful Light artwork palette and SDR photo
pipeline are untouched. Obsolete badge and three-card layout rules were removed.

## Verification

Production UI compilation and all 154 UI unit tests pass. Nine distinct focused
browser cases pass across the final and regression runs. These cover selection
and persistence, a single inert preview, no playback or metadata commands from
preview interaction, both preview views, clean portrait hover/focus, separate
keyboard-accessible attribution, artwork viewer focus restoration, retained Audio
settings, biography expansion/collapse, interruption and Reduced Motion.

The artist layout/contrast matrix passes 40 window/scale/theme combinations.
The picker matrix passes 48 combinations, checking all three presets in each:
720×480 and 1280×800; 80%, 100%, 125%, 150%; Light/Dark blue, Light/Dark artwork,
and extreme Light/Dark custom colors. Neither picker nor Settings panel acquires
horizontal overflow. Actual minimum dimensions remain 244×320, 220×120 and
320×120. Finite preview animations settle without leftover animation work.

The first portrait check exposed an older identity-transform rule; the still
portrait owner now overrides it explicitly. A fixture credit test initially
injected against a source comma removed by the dev transform; its injection was
corrected and the complete attribution/focus check passes.

Browser screenshots are under `out/persistent-qa/` in the isolated desktop
worktree. Native build, installation, preservation and screenshots follow below.

The optimized native build and strict signature verification pass. The signed
executable SHA-256 is
`3d912baa8e9458060ab39e1d94c86ff4249c9e867c8f22f5fe546a53ebe9612d`.
All eight embedded runtime/model files are byte-identical to the last signed
build (`72857fa7…`). The prior bundle is retained at
`releases/desktop-declutter-2026-10-25/previous-MusicOS.app`.
The running application's original bundle path was already absent when packaging
began, so the matching signed isolated bundle was preserved before rebuilding.
No OS implementation files changed. Installation followed a fresh native paused check after the Mac was unlocked.
The latest private profile copy and a verified SQLite backup were made before
loading the app; the previous signed bundle remains available.


## Native verification and preservation

MusicOS is installed and visually checked at the saved 125% interface scale,
Light artwork appearance, 150% saturation and 15% tint. Compact remains selected.
Settings shows a single bounded preview, its current cover and song, with the
size description/dimensions shown once. Artwork / Up next switches the inert
preview while the main player remains paused at 1:08. Artist photos no longer
acquire badges; Photo credit expands separately with PhilipRomano and
CC BY-SA 4.0, then closes cleanly. The portrait opens the viewer, Escape closes
it and restores portrait focus, with no badge overlay.

A native startup check exposed a missing current-album source cache image.
The running prior app had retained the cover in memory, while its saved PNG was
absent on disk. That one missing cache file was reconstructed from the embedded
cover in the same local album's first track. Audio SHA-256 was checked before
and after, and no audio or database changes were made by the recovery. The cover
now displays in Now Playing and the preview after restarting while paused.
Other cache files and user artwork were not replaced. The cache recovery record
is `out/qa/os-foundation/declutter/cover-recovery.json`.

Playback queue, index, volume, shuffle and repeat match the immediate pre-install
checkpoint exactly. Position is 68.73248 seconds after the native seek roundtrip,
versus 68.7325 before: a 20 microsecond change, less than one 44.1 kHz audio frame.
The visible paused position remains 1:08. All six collection tables, favorites,
interface preferences and integration configuration match: 508 songs and 34
favorites remain. Signed executable and all eight protected runtime/model hashes
pass after installation. The app is left paused on Travis Scott's page, with
credits collapsed and portrait focus restored.

Native screenshots and final check logs are under
`out/qa/os-foundation/declutter/`. The release manifest, protected hashes,
post-load checks, prior app and private-checkpoint pointer are retained under
`releases/desktop-declutter-2026-10-25/`. Source implementation is isolated commit
`8677978`, mirrored into the OS worktree as UI-only commit `4977322`.
