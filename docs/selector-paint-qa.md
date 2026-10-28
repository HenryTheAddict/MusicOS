# Selector choice painting — 2026-10-27

Native reproduction matched the reported screenshot: the rounded Library shell expanded but Devices and Settings were blank. Both items still existed in the accessibility tree. The adaptive header selector paints at stacking layer 21, while the portaled dropdown inherited layer 20. Its shell therefore covered the menu's text. The new shared dropdown layer is 100, above the header shell; measurement, clipping, 140 ms reversal, focus and delayed navigation remain unchanged.

A raster regression failed before the fix with no visible text-colored pixels. It now counts actual painted glyph pixels inside both choice-label bounds across 80/100/125/150 percent interface scales. This catches the occlusion that DOM visibility and background-color sampling missed. Intermediate-frame tests also cover opening, collapse, and reversal. Existing connected-collapse, menu replacement/focus, closed paint and rounded-trigger tests remain in place.

The Dock shortcut's stale MusicOS label and old-case URL were refreshed to musicOS at the same position. Its stale bookmark was removed so it resolves the current app; other shortcuts and Dock preferences were preserved, with a private checkpoint before the change. Native bundle name, display name, main window, app menu and wordmark remain musicOS; bundle/profile identities are unchanged.

The headphone recovery build was installed after the user paused Nangs at 62.9 seconds. This selector repair is delivered on top of that build, retaining its output reconnection changes. Latest profile, previous app and eight protected runtime/model hashes are preserved in the new delivery checkpoint. Physical headphone switching remains a user acceptance check.
