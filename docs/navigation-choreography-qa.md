# Library navigation and theme surfaces — 2026-10-19

The outgoing Library toolbar now fades and lifts out over 140 ms when opening an album or artist. Navigation and final layout update immediately: a short-lived, inert, accessibility-hidden visual copy provides the exit without reserving a blank row or delaying input. Open filter popovers are not retained. Back restores the actual toolbar with a 180 ms entrance. Rapid navigation cancels the outgoing copy.

Album artist avatars appear separately with a short scale/position reveal, followed by the primary credit. Artist names reveal by whole words; portraits opened without a grid origin have a small 300 ms entrance. Grid-origin portraits retain their shared artwork transfer rather than adding another fade. Supporting artist information follows in order. These are finite transform/opacity effects: no new idle loop, scroll interception, blur, gradient or data-completion animation. Reduced Motion skips these effects, and enabling it cancels in-flight motion.

The top section selector and its menu now share a solid control surface in artwork/custom themes rather than using a full accent fill. The Settings category background derives from the page surface with a small ink tint, avoiding an unrelated blue accent wash. The page outline uses the existing separator token. Native inspection also identified the default-blue fallback for muted artwork: artwork accents now retain that artwork's quiet hue, with a neutral accent for monochrome images. Original blue/custom choices and canonical saved artwork palettes are retained.

## Automated verification

- 102 UI unit tests pass, including muted green/monochrome accent and contrast coverage.
- 22 unique browser scenarios pass in focused batches: the 10 existing album/artist motion checks, three navigation regressions, five new toolbar/artist/theme checks, and four artwork-theme fade regressions. The first theme assertions incorrectly expected a legacy selected-header element; final tests use the current menu and retain a playing cover through Settings navigation.
- Coverage includes pointer/keyboard opening, interrupted artwork return, resizing, compact windows at all four scales, Reduced Motion, no navigation replay after artwork invalidation, Filter popover cleanup/focus restoration, and Light/Dark navigation surfaces with a deliberately mismatched green/blue provider palette.
- Production UI compilation passes. Light/Dark Settings and artist screenshots, including an in-flight artist reveal frame, were inspected separately from geometry assertions.

Browser fixtures verify behavior and layout; sustained native 120 Hz pacing and subjective motion/design acceptance are not established by these tests. No audio processor, queue membership, metadata, collection or original file changes are part of this update.

## Package and native delivery

Final executable SHA256: `6f0475606f554dd9f8c2803fdc7de8e144387a70c52fa221a3c37add8748ebf5`. Ad-hoc signature passed strict deep verification; this is not a Developer ID/notarized package. All 12 current JavaScript chunks were verified embedded. Playback runtime SHA256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

Loaded into main MusicOS only after verifying paused playback. Both reloads retained Labor of Love by Theodore Shapiro at 124 seconds, 507 library songs and 17 favorites. Native inspection confirmed the Filter bar leaves the album detail layout, Junk opens with readable artist credits, M83 opens with its portrait/name, and the full biography expands beneath the portrait. Settings was inspected in the user's current Light/artwork theme at 125% interface scale, 150% saturation and 20% tint; the large selector and category surfaces match the artwork theme. The final reload additionally confirmed the formerly blue rail/Create playlist accent now follows the muted green artwork.

The native mini-player artwork and empty-queue views showed centered tools/transport controls, kept the paused source position, and returned to main MusicOS. Its artwork preference was restored after the queue check. A populated native queue's scroll retention and physical 120 Hz pacing remain outside this check; browser queue regressions are recorded in the preceding player QA. No favorite, membership, file, metadata or appearance changes were made. The final app is left in the original Albums view, paused. QA profiles were preserved.
