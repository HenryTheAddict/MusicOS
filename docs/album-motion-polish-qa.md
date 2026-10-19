# Album motion and listening color polish — 2026-10-18

Album artwork now moves from the clicked grid tile or artist album strip into the detail header over 320 ms, with a bounded cached image and an ease-out curve. Titles, artist credits, details and actions reveal in short groups (200–220 ms, at most 135 ms of stagger). A single song-row group keeps long albums inexpensive. Returning to the grid or artist strip reverses the cover connection; interrupting it uses the currently visible position.

Only deliberate navigation starts these effects. Metadata refresh, playback position and artwork decoding do not replay them. Overlay images are non-interactive and hidden from accessibility. Resizing, scrolling, leaving the view, or Reduced Motion cancels and releases the transient image. There is no full-grid geometry pass, scroll interception, idle frame loop, new animation dependency, or artwork-completion flash. Normal images remain visible after cancellation and during cache preparation.

The artist detail portrait is now a still photograph: removed its competing pointer spring and depth compositor, plus image rotation/zoom. Hover/focus information remains, with flat tags and a short reveal. Grid and expanded-artwork depth still follow the existing preference, caches and memory budgets.

Artwork accents keep their hue but use a controlled brightness rather than the native palette's deliberately dark canonical shadow color. Canonical saved palettes and artwork are untouched. Playback, Play and More actions use neutral reading ink. Shuffle uses a consistent solid theme surface, with readable cover-derived ink in light collections and neutral ink in Dark mode. Column labels and Back controls now derive their reading colors from the theme. Existing blue/custom choices and charcoal Dark surfaces remain.

## Verification

All 99 UI unit tests pass, including appearance contrast across extremes, original-palette preservation, artwork caching and icon geometry. TypeScript and the production frontend build pass.

Focused browser tests cover click-to-header geometry, stagger timing, interruptions, return to an artist strip, keyboard entry, live Reduced Motion changes, resize cleanup, no replay on artwork invalidation, flat artist photos, matching Shuffle/More surfaces, actual label/Back contrast, and 720×480 at 80/100/125/150% interface scale. Screenshots inspected separately at 720×480 and 1280×800, including Light and Dark.

All 19 focused browser tests pass, including the depth/navigation regressions and Dark corner pixels at 1×/2×. Logs: `out/persistent-qa/motion-polish-unit.log`, `out/persistent-qa/motion-polish-verified.log`. The six focused motion/contrast tests were rerun after the saturation correction and pass at both 100% and the user’s saved 150% saturation. Log: `out/persistent-qa/motion-polish-acceptance.log`. Final signed-build identity and native checks are recorded below.

Human motion/design acceptance, physical touchscreen behavior and native 120 Hz frame pacing are separate gates. Windows/Linux are unverified. Reverb, queues, collections, original files and metadata were not changed by this task.

## Signed package and native checks

Final executable SHA256: `b265a79ac19b281589cc94d77a5bdd0e1506b9ac3bde7754b45749e2618a0784`. Ad-hoc signature passed strict deep verification; all 12 current UI JavaScript chunks were verified embedded. This is not a Developer ID/notarized package.

Embedded playback runtime SHA256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

Loaded the final package with playback paused on Go! by M83 at 62.5 sec. Inspected native Fancy That detail in Dark mode after the saved-150%-saturation correction, and the steady M83 portrait in restored Light mode. Actual reading labels, neutral transport controls, matching action surfaces and artwork display were visible. Original appearance restored: Light, artwork colors, 150% saturation, 20% surface tint, 125% interface size. Library remains 507 songs / 17 favorites.

macOS locked before the final optional Back to Artists action completed, so that view restoration is unverified; the last verified screen is M83 detail. The final build is running and paused. Native screenshots establish static presentation; motion geometry/reversal/timing were verified automatically, with subjective motion acceptance still reserved for the user.
