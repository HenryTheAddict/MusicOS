# Connected desktop motion, mini-player, and mix discovery — 2026-10-22

Desktop implementation: `20ef2a3`; integrated with the OS branch as `a156646`. Desktop UI changes only: no database migration, playback command change, inference-model replacement, or embedded runtime rebuild.

## Delivered behavior

- Personal Home stays first as a single rounded entry with a separate creation +. The + exposes the existing manual, audio Sparklist, and Smart playlist editors; More only navigates.
- Collection palette ownership includes Home cards/recommendations, management rows, and detail controls. Cached artwork palettes drive locally shaded playback glyphs; contrast is calibrated through the existing transport-tone helper. Theme changes recalibrate controls without changing saved artwork.
- Mini-player layout has one stylesheet. Two retained artwork/queue compositions crossfade over 180 ms, reversing through CSS's current visual state. Both views share fixed, centered 40 px footer controls. Queue identities/scroll and the native window size remain intact. Track snapshots crossfade for 180 ms; position ticks do not create snapshots. Menus and queue controls morph over 140 ms; Play/Pause retains 360 ms.
- Pointer menus retain their opening coordinates through exit. Menu actions run after closure and focus restoration, allowing creation/review dialogs to restore their real opener. Home and library album actions share a builder; track and collection actions serve Home, rows, and management. Queue keyboard context targets its row instead of the whole mini-player. Deleting a collection requires review and leaves music files alone.
- Home artwork does not enlarge on hover. Source/return identities allow its ordinary album transfer to return to the correct card; transitions release on resize/scroll. Group entrances are short and navigation-only.
- User clarification added **new mix ideas plus saved mix recommendations**. Local seed ranking weighs current music, recent listening, play counts, and favorites, with artist/album diversity penalties. Choosing a new idea opens the existing cached-audio generator and previews once setup/coverage permit. No mix is saved, refreshed, or played automatically. Missing setup remains actionable. Saved mix ranking still uses its existing audio/recent/favorite algorithm; neither path rewrites saved sequences.
- Album descriptions occupy a two-line preview beside quality/play controls. Read more opens attributed full text separately and restores focus. Artist previews use four lines with their existing full-reading action. Duplicate search is removed from Artists and artist-detail chrome; Command-K remains available.

## Verification

Production TypeScript/Vite build and 116 UI unit checks pass. Browser evidence includes the 36-check desktop regression suite; 12 focused checks for new behavior, queue context, pointer/keyboard resize, retained Settings, secret clearing, and remembered mini-player views; plus five final description/mix checks and the forty-case header/Home layout matrix. Some checks overlap between runs. Home and album-header layout matrices cover 720×480 and 1280×800, all four interface scales, and Light/blue/artwork and Dark/blue/artwork/extreme custom appearances.

Inspected fixture screenshots include the 244×320, 320×430 and square mini-player artwork/queue views, long album descriptions, and Light/Dark themes. Corner pixels in the 1× and 2× transparent mini-player captures have alpha zero at all four outer corners. Paused mini-player tests confirm no running subtree animations after finite feedback settles. Reduced Motion, rapid queue reversal, retained row identity/scroll, favorite tempo behavior, collection deletion Cancel, and model/Settings polling behavior are covered.

Disposable browser fixtures were used for creation, deletion review, metadata drafts, and queue mutations. No real music files, tags, credentials, saved sequences, or library records are edited by automated QA.

Retained logs/screenshots: `out/qa/os-foundation/connected-motion/`. Early failed runs record real focus/coordinate issues and obsolete test expectations; final passing results supersede them.

## Native delivery

Installed and verified the ad-hoc signed package after confirming On Track paused at 221.3 seconds and quitting normally. Fresh private preferences and the actual previous bundle are retained in the release/checkpoint directories. No older state was restored. Strict deep signature verification and all installed hashes passed.

Executable SHA-256: `fb554ab9fe7c2d2c3dd27e6f403f702fc95e653074bbd667df99f87e8bc458a9`.

Unchanged embedded hashes:

- `musicos-mac`: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

Native Home shows new seed-based previews and ranked saved mixes with collection-specific playback shading. Library counts are 507 songs, 26 favorites, and 12 collections, including Mm. Artwork and queue mini-player layouts, five upcoming entries, close/reopen, the pinned-state actions menu, and return to main were inspected. Creation exposes all three editors and restores focus on Escape. The compact description sits beside controls; full attributed text opens separately and Escape restores Read more focus. Native screenshots are retained with the evidence.

The user interacted during verification and advanced On Track to 233 seconds; the latest state was left paused on Home's recommendations. One UI-control call timed out and was recovered by refreshing the native binding; no forced quit or stale-profile restoration was used. Automated pointer/keyboard resize checks pass; measured native resize/drag displacement, cross-monitor behavior, and physical-display frame pacing remain acceptance gates.

Physical 120 Hz packaged frame-interval measurements, subjective motion/design acceptance, real touchscreen interaction, and Windows/Linux remain separate acceptance gates. Headless browser timing is not claimed as physical-display performance.
