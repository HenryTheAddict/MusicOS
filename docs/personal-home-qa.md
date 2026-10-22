# Personal desktop home — 2026-10-22

Implemented on isolated desktop commit `121e4ef`; integrated on `aa4a1cf` while retaining the OS branch’s connected-device command and extracted-core database adapter.

## Interface

- The display-name button (for example, **Henry’s stuff**) is the first sidebar destination and opens a personal home. New navigation sessions start there. Existing valid saved routes remain valid.
- Home includes a current-song Resume/Pause control, **In rotation**, **For your mood**, **Recently played**, Favorites, and saved collection shelves filtered by All / Playlists / Sparklists / Smart playlists. Play controls are separate from collection-open controls. Existing sequences, creation, details, generation, and rule workflows are reused.
- Albums, Artists, Genres, Songs, Playlists, Sparklists, and Favorites remain distinct sidebar destinations underneath Home. Up to three manual playlist shortcuts remain available for sidebar drag-and-drop; home manual collection tiles also accept the existing song-drop flow.
- Collection filters are remembered in session storage. Navigation retains the home scroll when returning from details. Album opening uses the existing artwork-transfer animation and cached ordinary images. Existing mini-player motion, hearts, and player styling are unchanged.

## Listening data and loading

The new `home_activity` command only reads existing song play counts and last-played timestamps. It returns up to 200 frequently played song IDs/counts and twenty recent IDs. Home aggregates available songs by canonical album/edition ID, displays six albums and six recent songs, and filters missing/unavailable records. There is no new history store, schema migration, remote service, or background inference.

With no listening history, the home labels its selection **From your library**, rather than inventing frequently played music. The first read uses a quiet fixed-geometry placeholder; later entries reuse shared cached display data while refreshing. Errors retain cached content or the ordinary library selection with a short explanation. Requests occur on entry, current-song changes, or library changes; they do not follow position ticks and stop when the view is inactive. Recommendations reuse the prior saved-Sparklist ranking and consent.

## Checks

- 115 UI unit tests passed, including edition identity, unavailable-history filtering, cold-start behavior, default Home, saved legacy routes, and return navigation.
- 146 Tauri unit tests passed, including two new listening-summary ranking/bounds cases. The two new cases also pass against the primary extracted-core integration.
- Ten final browser cases passed. The home layout case inspects forty combinations: 720×480 and 1280×800, 80/100/125/150% scale, and Light/blue, Light/artwork, Dark/artwork, Dark/blue, Dark/custom appearances. It checks horizontal bounds and collage clipping. Other cases cover home-to-detail return/filter retention, unchanged saved collections, visible/hidden request behavior, Resume without queue replacement, Settings drafts and credential clearing, mini-player fixed controls/remembered view, and Command-K queue opening.
- The Resume fixture has twelve queue entries including the playing song and eleven upcoming rows. The initial test incorrectly expected twelve upcoming rows; the corrected assertion passed.
- Browser fixture preview is working and left open on Home. It has no saved mixes or listening history, so it correctly shows the library fallback and collection empty state. Seeded fixture screenshots demonstrate the populated shelves. All collection/playback mutations in automated checks used disposable fixtures.

Evidence: `out/qa/os-foundation/personal-home/`. The preview server runs from the isolated desktop worktree on port 1420.

## Signed build and native delivery

The candidate is retained at `releases/desktop-personal-home-2026-10-22/MusicOS.app` with a copy of the previously installed app. Ad-hoc signing and deep strict signature verification passed.

Executable SHA-256: `cd66041fa0036b89a6d16fe05fb47b0e5157e5253af098beab9fbd738097b6a3`

Unchanged embedded components:

- `musicos-mac`: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

**Installed and verified after the user unlocked the Mac.** Before replacement, native controls showed PILOT paused at 147.3 seconds, 11 upcoming songs, 507 songs and 22 favorites. MusicOS was quit; latest preferences were checkpointed under application-data `Development/checkpoints/2026-10-22-personal-home-install/`. The app actually installed at publication time is retained as `Pre-install MusicOS.app` in the release directory. Publication used a hash/signature-verified staging bundle and rename while the main process was stopped.

Native Home displayed the real listening data: six frequently played albums, three recommended saved Sparklists, recent songs, Favorites, and the saved collection shelf with **View all 11 collections**. The manual playlist and ten Sparklists were retained. Startup showed the same PILOT song; the user then resumed playback, browsed the Home album shelf, and selected UTOPIA music during verification. Latest observed playback was MODERN JAM. Further UI changes stopped so the user’s latest playback and browsing were preserved; no claim is made that playback stayed paused during their interaction. Native queue-order comparison after that interaction was not attempted. Isolated fixture checks cover queue preservation and resume semantics.

Installed executable and embedded hashes were rechecked after publication; strict signature verification passed. `release.json` records installed and loaded/verified status.

Subjective design acceptance, native motion/resize checks, physical touchscreen behavior, Windows/Linux, and packaged frame-interval measurement on a physical 120 Hz display remain separate gates. No frame-rate claim is made.
