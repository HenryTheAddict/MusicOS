# Artist reading, player alignment and motion work — 2026-10-18

Artist biographies retain their full text and paragraph breaks. The portrait-side preview uses an eight-line clamp without nested scrolling. Read more stays outside the clipped text and expands the biography into a full-width row beneath the portrait, with a 65-character reading measure, normal wrapping and Read less. Descriptions that already fit need no expansion control. Resize observation measures only that paragraph, never the full library.

The mini-player keeps its existing design and menus, as clarified by the user. Its tool and transport icon hosts now center inside their buttons rather than following an inline text baseline. Main Now Playing uses a shared vertical frame for artwork, title/credits and rail. Text remains left aligned beside the cover at all four interface scales.

Playback position/level events formerly remapped the full queue and rebuilt its rows even when membership and order were unchanged. Queue presentation now retains its identity until membership/library metadata changes; unchanged numeric pin/arrangement/entry arrays are retained on event merge. Mini-player queue content updates only for queue/index/mark changes and keeps its DOM/scroll while hidden. Genuine edits and duplicate entry identities remain authoritative. The Escape listener also stays installed across clock updates.

Detail pages no longer combine a full-page opacity fade with their own staged copy/artwork animations. Shared artwork motion and the text/group reveals remain; grid and Settings navigation retain their existing treatment. No animation library, frame loop, scroll interception or artwork-cache change was added.

## Comparative profiling

Isolated headless Chromium development fixture, 90 queued entries, five two-second runs of native-shaped 20 Hz playback events, same machine/profile and viewport. React Strict Mode is enabled in both versions; this is not a native 120 Hz measurement.

| Median per two-second run | Before | After |
|---|---:|---:|
| Queue renders | 80 | 0 |
| Main-thread task time | 780.58 ms | 470.64 ms |
| Script time | 684.70 ms | 398.62 ms |
| Style time | 8.95 ms | 10.06 ms |
| Layout time | 5.84 ms | 6.35 ms |

Main-thread task work fell 39.7%. The browser ran at approximately 60 Hz (median frame interval 16.7 ms in both versions); native sustained 120 fps is not claimed. Raw reports: `out/persistent-qa/player-motion-before.json` and `player-motion-after.json`. This improvement addresses repeated playback work, not every possible source of native animation hitches.

## Verification and package

All 101 UI unit tests pass. Final focused browser coverage has 31 passing scenarios across the motion/navigation regressions, mini-player behavior, metadata/choice regressions, and 11 reading/alignment/queue checks. These were run in focused batches, including complete text visibility when a biography already fits without expansion. Screenshots were inspected separately at compact and wide sizes. All four interface scales pass reading and title/cover geometry checks. Keyboard expansion, long words/paragraphs, queue reorder/removal/promotion and retained queue scroll are covered.

The comparative profile is opt-in and records separate performance artifacts; production UI compilation passes. No original files, metadata, saved collections or reverb processing changed. Human motion acceptance, actual native 120 Hz frame pacing, physical touchscreens and Windows/Linux remain separate gates.

Executable SHA256: `b75b0ea561ab692ecbfcea0de633abf2f4063a5e6dae7fc4259f8cdbe2b10fab`. Ad-hoc signature passed strict deep verification; all 12 current JavaScript chunks were verified embedded. This is not a Developer ID/notarized signature. Playback runtime remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

Native delivery — 2026-10-19: after the user unlocked the Mac, MusicOS was verified paused on Go! by M83 at 62.5 seconds, then quit and reopened from the signed bundle. The reopened window retained that paused source position, 507 songs, 17 favorites and saved collections. A native screenshot showed the updated left-aligned Now Playing title/credits and rounded rail beside the artwork. The app was left in its original Albums view; no playback, queue membership or appearance changes were sent.

The next accessibility read failed with cgWindowNotFound; reconnecting reported that the Mac was locked. The full native biography expansion and mini-player alignment/queue checks therefore remain pending. Browser reading/alignment/queue verification above is complete, but does not substitute for native motion acceptance or a physical 120 Hz measurement.

Follow-up native verification on 2026-10-19, during the navigation/theme update: M83's full biography expanded into the wide row below the portrait and retained Read less. The native mini-player artwork/queue views showed centered tools and transport buttons, retained the paused 124-second Labor of Love source position, and returned to the main window. The queue was empty at that point, so this does not establish native queue-scroll retention with a populated queue. No memberships, favorites, metadata or appearance settings were changed. Physical 120 Hz pacing remains unmeasured.
