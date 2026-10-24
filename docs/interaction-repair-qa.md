# Desktop interaction repair — 2026-10-23

## Changes

Playback now publishes the clock, meter levels and processed-frame counter to small visible consumers. The library receives track, queue, transport and configuration changes; position/meter ticks do not reconstruct pages or menus. Progress remains seekable, meters remain live, and acknowledged Play/Pause changes still drive the existing 360 ms morph. Hidden consumers unsubscribe. Artwork observation ignores unrelated SVG/label mutations. Grid depth rendering uses a bounded 2-megapixel backing store (up to 1.5× density), while ordinary covers retain their original image resolution; hero rendering has a separate 4-megapixel budget. Shader output and artwork identities are unchanged.

Action menus use a compact, fully rounded floating panel at the pointer or button, with grouped actions and no artificial connecting strip. They do not lock scrolling or pointer events on the page. Selection retires the panel immediately before executing its action; Escape/outside dismissal retains a short opacity exit. This also prevents an outgoing menu from consuming the new dialog's Escape key. Keyboard navigation and focus restoration remain supported. Long-label hover starts after 120 ms and finishes within 1.6 seconds rather than waiting half a second and scrolling for up to 12 seconds.

Show in Finder is available for songs, Now Playing and queue entries. The native command accepts a library track ID, resolves its current file from the repository, checks that it exists and reveals it using argument-safe platform commands. Missing songs return an error; the command never opens a media file for playback. macOS reveals the selected file; Linux opens its parent folder and Windows uses Explorer selection. Non-macOS behavior remains a separate native verification gate.

Standard retains 244×320. Compact is reduced to 220×136, with less outer padding and top-aligned compact information. Wide is an explicit 360×144 preset with a 72 px cover and one control row. Each preset has a still Appearance preview, separate session bounds and matching native/frontend minimums. Artwork/queue switches retain size and queue identities/scroll. The small preset queue header is compact enough to leave a complete upcoming row visible. The existing optional density preference now accepts `wide`; no database migration or playback protocol change is included.

Smart playlist dialogs enter with a finite 180 ms transition. Added rules fade in and remaining rules move into place on removal, retaining stable rule identities, values and focus rather than remounting adjacent inputs. Reduced Motion skips spatial animation.

## Verification

- UI production compilation, 135 UI unit tests and 155 Rust library tests passed.
- Focused interaction checks cover clock updates without library renders, progress advancement, pointer positioning, unlocked page interaction, Escape/focus, Finder action exposure, Wide control alignment, retained queue bounds and Smart rule values/animation cleanup.
- Minimum-size geometry checks passed for Standard, Compact and Wide at 80/100/125/150% interface scales in Light and Dark (24 combinations), including a complete upcoming row at each minimum.
- Regression checks cover menu/dialog creation flows, keyboard context entry, Home cover bounds and action parity, independent collection colors, Play/Pause midpoint/reversal/Reduced Motion, retained Settings previews, track changes and queue focus.
- Mutation tests used disposable browser fixtures. No music files, tags, saved collections or OS source were edited.

A menu creation-flow regression initially failed because an outgoing menu still consumed Escape. The corrected action path overrides the CSS exit animation before unmounting, then runs the action. The checks explicitly verify that all three creation dialogs close and restore focus to the creation button.

## Comparative performance

An isolated copy of the immediately preceding desktop source was compared with this frontend in headless Chromium at 1280×800. Each build received three two-second samples of 20 Hz native-shaped playback position, meter and counter updates, with the same library grid. These are browser scripting/layout measurements, not native display pacing or a 120 fps claim.

| Median per two-second sample | Before | After |
| --- | ---: | ---: |
| Grid renders (development Strict Mode) | 80 | 0 |
| Total task work | 477.57 ms | 188.56 ms |
| Scripting | 406.85 ms | 62.41 ms |
| Style work | 11.22 ms | 20.25 ms |
| Layout | 7.50 ms | 12.77 ms |

Total task work fell about 60.5% and scripting about 84.7%. Style/layout rose by about 9.0/5.3 ms per sample; that increase is included in total work rather than omitted. Native frame presentation and human motion acceptance remain separate gates. The temporary baseline server was stopped after measurement.

Evidence: `out/rebuild-qa/interaction-*.log`, `interaction-profile.json`, `wide-player-artwork.png`, and `interaction-menu-preview.png`. Signed native delivery and state preservation are recorded after completion below.

## Installed native build

Desktop source `5948aa8`, copied to the shared foundation as `3a58085`. The compatibility merge retained the OS branch's existing `connected_device` command alongside the new Finder action. The primary desktop compatibility check passed. Embedded runtime/model source and binaries were not rebuilt or replaced.

Signed executable SHA-256: `ebe95645c4cc0f151b01f0e886948d4e3e1799fc37d0effa70448bb12d822b23`. Ad-hoc signing and strict deep signature verification passed. The full five-resource manifest is retained with the dated release; its embedded runtime, ONNX library and three model resources match the preceding release exactly. Runtime: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

A fresh paused-state check preceded clean quit, private JSON/profile copying and a SQLite backup verified with `quick_check`. The previous app is retained in `releases/desktop-interaction-repair-2026-10-23/previous-MusicOS.app`. The replacement restored PILOT at 195.7 seconds, paused, and 508 songs. Integration configuration bytes were unchanged.

Native Show in Finder selected the actual PILOT FLAC inside its album folder. The Wide window measured 720×288 image pixels at 2× density, matching 360×144 logical units; artwork/queue switches retained that size and the same bottom controls. Actions/menu return and retained Settings were exercised. Wide was selected for the live check. Native screenshots are retained under `out/qa/os-foundation/interaction-repair/`; macOS window-sharing indicators are system overlays rather than MusicOS artwork.

The final focused browser regression run passed all 12 cases. The 24-combination minimum geometry run, 135 unit tests and 155 Rust tests also passed. During native checks the user continued interacting: Mm changed from two to three songs, and playback/navigation changed. Those latest edits were retained and captured in a second private verified backup; the pre-load database was never restored. Saved art, playlist tombstones and details remained equal, while playlist membership and modified timestamps reflected the live edits. No native collection edit or playback command was performed for the smoke check.

Native hover frame presentation was not instrumented; the measured scripting reduction is the browser comparison above. Human interaction/motion acceptance, physical touch, and Windows/Linux native checks remain separate gates.
