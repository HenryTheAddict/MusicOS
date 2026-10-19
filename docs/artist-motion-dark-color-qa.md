# Artist transitions and colored Dark surfaces — 2026-10-18

Artist cards now connect their existing cached portrait to the detail portrait over 320 ms. Heading, biography, statistics, album strip, and song group enter over 220 ms with 30–135 ms delays. Returning to Artists reverses the connection; an immediate reversal starts from the current displayed portrait bounds. Artist detail portraits remain steady afterward, without an additional pointer/depth transform. Direct navigation without a visible portrait still reveals text without inventing an image origin.

The same bounded transition lifecycle handles albums and artists. It retains the source image framing, hides only the destination image during transfer, and restores it on completion, resize, scrolling, navigation, or Reduced Motion. Provider/data refresh does not replay navigation motion. No new dependencies, idle loops, full-grid measurement, artwork cache changes, or inference work were added.

Dark mode now gently mixes artwork hue into solid charcoal page and shell surfaces. Surface tint controls the amount; zero tint and monochrome artwork stay neutral. High saturation cannot overwhelm the surface tint. Text contrast is at least 7:1 against computed page/shell surfaces, with accent contrast checked against both. Controls, the player, and portal menus use the same tinted surface family instead of fixed gray backgrounds. Saved canonical artwork palettes and Light mode remain unchanged. No gradients or pale inset highlights were added.

## Automated and visual verification

All 99 UI unit tests pass. These include contrast extremes, bounded dark tint, zero-tint/monochrome behavior, canonical-palette preservation, image caching and existing queue/metadata controls. TypeScript and production UI compilation pass.

Browser checks cover artist keyboard entry, source-image identity and bounds, staged copy, rapid reversal, static detail portrait, no animation replay on artwork invalidation, resize/Reduced Motion cleanup, album regressions, adaptive color switching, retained Settings/library resources, and Dark corner pixels at 1×/2×. Separate screenshots inspect tinted artist detail and portal menu surfaces, including compact layouts. All 15 focused browser checks pass (14 motion/navigation/edge checks, plus compact artist detail at 80/100/125/150% interface size). Screenshots were inspected separately at 1280×800 and 720×480. Logs: `out/persistent-qa/artist-color-unit.log`, `artist-color-browser-verified.log`, and `artist-color-compact.log`. Signed package identity and native results follow below.

Playback, reverb, metadata, source files and saved collections were not changed. Human motion/design acceptance, physical touchscreen behavior, native 120 Hz pacing and Windows/Linux remain separate verification gates.

## Signed package and desktop delivery

Executable SHA256: `ec9ef4d7272affd255562d9adb1c10868a87c090e36650d2a203e64438326419`. Ad-hoc signature passed strict deep verification; all 12 current UI JavaScript chunks were verified embedded. This is not a Developer ID/notarized signature.

Embedded playback runtime SHA256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`. The runtime preparation/replacement step was deliberately excluded from packaging.

Loaded the final package after verifying playback paused on Go! by M83 at 62.5 seconds. Native M83 detail now shows a subdued pink/rose charcoal surface derived from its actual concert portrait, replacing the former neutral gray. Inspected the ordinary portrait, reading controls and player surface; Back to Artists remained usable. Browser checks establish animation timing and interrupted geometry; native screenshots establish static presentation, not native frame pacing or subjective motion acceptance.

Restored the original appearance after QA: Light, artwork colors, 150% saturation, 20% surface tint and 125% interface size. Final screen is Artists with its scroll position retained. Playback remains paused at 62.5 seconds; 507 songs and 17 favorites remain visible. The user's QA profiles were not replaced or modified.
