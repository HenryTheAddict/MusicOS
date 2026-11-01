# Sparklist interface repair — 2026-11-01

## Scope

Creation now separates choosing a song/feeling, reviewing the mix, and naming/saving. A wider responsive preview pairs the generated cover and mix statistics with a numbered song list. Cover customization lives in the save step, with advanced color controls disclosed. Status identifies the current operation and reflects actual model/preparation state.

Saved Sparklist pages have compact Edit details / Refresh mix actions, with sequence conversion and confirmed deletion in the existing connected overflow. Details and artwork use a grouped editor, explicit cancel/save, and operation feedback. Existing generation, readiness/consent gates, saved sequences, queues, artwork choices, models, and playback commands remain unchanged.

Track lookup and the displayed generated cover are memoized. No new polling or animation loop is introduced. Existing idle-status polling stops when ready. Stage motion remains finite at 180 ms; button feedback is 140 ms and respects Reduced Motion. The stage-change controls have separate identities so a Next click cannot become a form submission when the save button appears.

## Verification

- UI build succeeds.
- UI unit suite: 203 tests passed. Readiness/model gate, explicit preview playback, feedback generation, draft retention, explicit saving, and existing playback/motion coverage retained.
- Browser checks: 20 passed (19 core/matrix checks plus the saved small-window editor check). Matrix covers 480×480, 720×480, 1280×800; 80/100/125/150% scale; Light/Dark artwork, blue, and extreme custom colors, all three creation steps. Checks bounded dialogs, no horizontal overflow, reachable actions, explicit save, edit/cancel/delete confirmation, nested song selection, model gate, idle polling, and Reduced Motion.
- Manually inspected before/after fixture screenshots of creation, preview, save, and saved detail. No live library changes or playback commands were used for those fixtures.
- Recommendation quality and model comparison are outside this interface repair.

## Delivery

Isolated desktop branch: codex/desktop-heart-preservation. Signed executable hash, unchanged embedded runtime/model hashes, fresh paused/stopped gate, latest-profile checkpoint, previous app archive, and installed verification are recorded in the delivery manifest. Initial native verification confirmed all creation steps and a real 25-song preview, with playback paused at 192.4 seconds throughout. It caught an inherited two-column form rule in the saved editor; the follow-up explicitly stacks its fields and adds a layout regression. Final native verification confirmed the corrected single-column fields beside artwork, focused Name entry, unchanged saved details after closing, both overflow actions, and Escape focus restoration. Playback remained paused at 192.4 seconds. No live mix was saved, refreshed, deleted, or played during inspection. The two editor browser regressions also passed after the native correction. No Android schema, Wi-Fi manifest, database, or OS-source changes.

## Installed result

Final signed executable SHA-256: `5c6fbc81d6550c51ce2e22dc6509dfc94725717a82ee1f027f1e21ded314cde6`. Installed only after a fresh native paused check. All eight protected runtime/model files match the prior app. Both app/profile checkpoints are retained, including the original app before this repair. Final comparison preserved all 581 tracks, 44 favorites, manual playlists/entries, Sparklist sidecars, metadata history, lyric selections/revisions/exports, settings, and the primary OS checkout. No Android update was produced.
