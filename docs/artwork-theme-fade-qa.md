# Artwork theme fade repair — 2026-10-18

The prior CSS animated theme colors only while the launch marker was active. Later artwork changes therefore jumped directly to the new palette. The launch timer also depended on the palette, so navigation or delayed artwork could restart the long startup period.

Artwork changes now use a separate 420 ms ease-out transition for solid page, shell, control-family and accent colors. Browser transitions retain the current displayed color on rapid reversal instead of starting again from an old album's endpoint. The initial launch fade remains; its completion deadline starts once and no longer restarts on palette updates. There are no gradients, page opacity flashes, new animation dependencies, frame loops or extra palette requests.

Explicit appearance/source/saturation/tint edits, System Light/Dark changes, and Reduced Motion apply immediately. Reading ink stays stable within Light or Dark; accent labels use the contrasting polarity guaranteed by the generated artwork palette, avoiding an unreadable intermediate text color. Light artwork surfaces reserve a little additional contrast because perceptual color interpolation can briefly be darker than either differently hued endpoint. Canonical saved palettes and images remain unchanged.

The existing request cancellation guard retains the current palette while preparing another and prevents late results for an abandoned album from repainting the active view. Tests now exercise that guard with a delayed, distinct palette rather than identical fixture colors.

## Verification

All 99 UI unit tests pass. All 11 focused browser checks pass: Light and Dark fades after launch, exact current-color continuity on interrupted return, 20 ms samples of page/shell/accent contrast throughout representative transitions (at least 4.5:1), delayed response rejection and retained current colors, explicit settings and System changes, Reduced Motion, launch completion, no restart on resize, no gradients, navigation effects, and retained Settings/library images and textures.

Logs: `out/persistent-qa/theme-fade-unit.log` and `theme-fade-browser-verified.log`. Light and Dark screenshots were inspected separately; they establish static presentation, while automated animation samples establish transition behavior. Production UI compilation and final package identity are recorded below after delivery.

Playback, reverb, source files, metadata, collection contents, image cache formats and embedded runtimes are unchanged. Human motion acceptance, native 120 Hz pacing, physical touchscreens and Windows/Linux remain separate verification gates.

## Signed package and desktop delivery

Executable SHA256: `c6047ded07b561a41c743170c27bff3c0c2bf74e40c165890dc661de44100fec`. Ad-hoc signature passed strict deep verification. All 12 UI JavaScript chunks were verified embedded. This is not a Developer ID/notarized package.

Embedded playback runtime remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`. Packaging retained the embedded runtime without running runtime preparation/replacement.

Loaded the signed package after verifying paused playback on Go! by M83 at 62.5 seconds. Native Fancy That detail displayed its warm artwork palette with readable controls and ordinary artwork. A second optional navigation capture was interrupted by a window change; refreshed accessibility state showed Albums, 507 songs and playback still paused at 62.5 seconds, so no further navigation was sent. The latest visible state was retained. Light appearance remained visible; no appearance or playback preferences were changed for this native check.

Native screenshots confirm packaged presentation; exact fade timing, contrast samples, rapid interruption and delayed-response behavior are automated-browser results. Native frame pacing and human motion acceptance are not claimed.
