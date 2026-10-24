# Navigation, song interaction and Integrations — 2026-10-24

## Changes and causes

Artwork snapshots now use the visible image bounds. Returning artwork targets the visible library content, with the grid held spatially still while its cover transfers. Its 320 ms transfer stays separate from 180 ms content reveals. Album headers reveal their current credits/footer groups rather than obsolete selectors. Regular destinations enter from transparent; pages containing song tables use opacity only to keep measurement and pointer targets stable. Decode/theme updates do not replay navigation.

Context-open requests capture current animation state as ordinary menu triggers do. Opening another menu retires the previous panel immediately without restoring focus into its old row. Normal dismissal still reverses from displayed opacity/scale; selecting an action retains immediate retirement before the new workflow opens. Keyboard focus and Reduced Motion are preserved.

Song rows reserve the number cell for the Play/meter control. Titles and credits no longer slide underneath the pointer. Interactive controls do not bubble into row double-click playback or row selection. A pointer press remains a click until six pixels of deliberate dragging; capture begins only then. Escape, context actions, loss of focus and navigation cancel the gesture.

Playlist drops show song count and destination, preserve the visible selected order, omit existing/unavailable songs and reject rule-owned or generated collection edits. Pending writes wait for acknowledgement and report completion. Sidebar edge scrolling runs only during a drag and stops on cancellation. Manual playlist block moves show before/after insertion and avoid no-op writes. Multi-song queue drops use the existing single-song command sequentially, preserving every selected song and its order. No playback protocol or database changes were made.

Integrations has one introduction and two compact aligned service rows. Setup, privacy copy and diagnostics remain available in disclosures. A legacy adaptive selector had forced square backgrounds over the component's own rounded surface; that override is removed. Secondary text derives from its actual control surface, with measured contrast of at least 4.5:1 across the checked themes. The Application ID draft survives inactive visits; secret inputs still clear. Controls recover when a save finishes while the page is inactive.

## Verification

- Production UI compilation, 143 UI unit checks and 155 Rust library checks passed.
- Focused browser checks cover current-bounds cover reversal, stationary return targets, decode invalidation without re-entry, artist return, resize/Reduced Motion cleanup, menu replacement/focus, stationary song labels and click/double-click isolation.
- Disposable fixtures cover duplicate-skipping playlist additions, ordered multi-selection, cancellation, before/after block reorder, multi-song queue insertion and drag-only sidebar scrolling. Live playlists were not edited for these mutation tests.
- Integrations geometry/contrast checks cover 720×480 and 1280×800, 80/100/125/150% scales, Light, Dark and extreme custom colors: 24 combinations. Retained Settings scroll, cleared secret drafts and hidden polling checks passed.
- Deep grid scrolling/reversal checks cover small/large libraries and depth on/off. Their setup was corrected to explicitly enter Albums after the existing Home default, instead of looking for album-grid cards on Home.
- Browser evidence is retained under ignored `out/rebuild-qa/`. The updated Integrations screenshot waits for entrance completion. Native delivery is recorded below after loading.

This pass does not claim a native frame-rate improvement from browser checks. Native motion acceptance, physical touch and Windows/Linux remain separate gates. OS and embedded runtime/model work remain separate.
