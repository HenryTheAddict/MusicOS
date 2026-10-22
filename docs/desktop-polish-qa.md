# Desktop search and management polish — 2026-10-21

## Delivered changes

- Command-K and library Search share an interruptible 180 ms opening/closing transition. Closing retains the dialog briefly; rapid reversals continue from its visible state and retain the query. Reduced Motion switches directly. Albums has a search action beside Filter, and detail navigation uses the supplied search glyph. The small module warms after the first library paint.
- Settings uses category headings, supplied navigation icons, quieter selected states, clearer action hierarchy and panel-width wrapping. The Library subpage has its own quiet loading boundary, keeping the navigation visible. Visited categories, scroll, searches and drafts are retained.
- Music folders are grouped into a single list with paths and counts separate from status and actions. Organize and collections are distinct rows; removal guidance and model attribution are expandable. No real source records were removed or forgotten during QA.
- Idle and hidden depth setup no longer polls indefinitely. It refreshes when visible and polls an active download until completion. Cached status remains usable; an unknown first status does not display a misleading Download action.
- Album controls now participate in normal layout below credits. Guest-heavy releases retain the named lead artist and put all contributors in a keyboard-accessible horizontal strip. Tooltips stay anchored to their artist, with the bright inset and following animation removed.
- Play/Pause still reverses over 360 ms, but now samples the supplied SVG curves instead of changing to approximate rounded trapezoids. Endpoint widths match the assets and intermediate contours stay inside their bounds. Geometry is cached; animation work stops at completion.
- Depth hover resumes after native scrolling settles, including with a stationary pointer. Preparation caches, ordinary-image fallback, GPU recovery, and scroll geometry are retained.
- Mini Player has a labeled Up next region, clearer rows, stable footer controls and preserved queue focus. Artwork and copy share an alignment frame at normal sizes; short native windows use the full reading width, a larger cover and a compact footer. Pinning, resizing, context actions, queue identities and BPM hearts remain intact.
- The collection manager has a clearer Create action, count-bearing tabs, search and sort, readable artwork/name rows and per-collection menus. Delete keeps its explicit confirmation and Cancel leaves sequences intact. Smart rule editing stays directly available. Sparklist control shading is restrained and theme-aware; playback order and saved covers are unchanged.

Implementation commits: isolated desktop `6011610`, `cbff658`; combined OS branch `1bc0f69`, `e326bd8`. OS services, database schema and playback contracts were not changed.

## Verification

- Production UI build passed in both the isolated desktop worktree and combined repository. All 112 UI unit tests passed, including morph reversal/width bounds and visible/idle/hidden setup polling.
- 88 distinct browser regression tests passed across the focused runs and corrected rechecks. Coverage includes keyboard search, rapid reversal, Reduced Motion, source remove/restore on disposable fixtures, saved collection cancellation, queue focus and geometry, 1×/2× mini-player alpha, contributor/control separation, GPU recovery and hover after scrolling.
- Metadata checks cover 720×480 and 1280×800, four interface scales and five theme treatments. Manual drafts and selected subpages survive navigation. Album review applies only selected fields, keeps file writing off by default and permits Undo on fixtures. The collection-management sizing loop also covers both sizes, four scales and five palettes.
- The old management tests still targeted the superseded song-first suggestion UI. They now exercise the current album review and History flow. One new test initially read request diagnostics before application completed; it now waits for the visible completion state. Two broader runs also collided while cleaning the same Playwright artifact directory; both affected cases passed again using separate output directories. These failures are retained in the logs.
- Native inspection verified search opening/reversal/dismissal, Settings, saved collection types, artwork/queue switching, context/pin actions and return to the main window. It caught a narrow text column at the real 244×320 mini-player size; the final compact correction was rebuilt, signed, loaded and inspected there. Browser checks additionally cover 320×430, 390×540 and 400×320.
- Main-library QA used viewing and window actions only. Metadata, collection deletion, source mutation, and file operations were tested only against disposable fixtures. Playback stayed paused on Intro at 151.4 seconds. The app retained 507 songs, 21 favorites, one manual playlist and ten Sparklists. It was returned to the playing album in the main window, with the original artwork mini-player preference retained.

## Package

Final package and previous bundle: `releases/desktop-polish-compact-2026-10-21/`. The first polish candidate and its preceding app are separately retained in `releases/desktop-polish-2026-10-21/`. Latest private preferences were checkpointed before each paused replacement. Signing is local ad-hoc; deep/strict signature verification passed. This is not a notarized release.

| Component | SHA-256 |
| --- | --- |
| Final desktop executable | `7ddb252322b45ebaf325fdccac6a78933db0c6f6336ce742cfd8fa3951182af1` |
| Preserved embedded musicos-mac | `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d` |
| Preserved embedded ONNX | `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920` |

Evidence is retained in `out/qa/os-foundation/desktop-polish/`. Subjective design/motion acceptance, physical touchscreen interaction, complete native pointer/keyboard resize coverage, Windows/Linux, and measured packaged frame intervals on the actual 120 Hz display remain separate gates. Software WebGL and browser geometry checks do not establish a hardware frame rate.
