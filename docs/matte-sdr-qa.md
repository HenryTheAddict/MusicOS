# Softer artwork surfaces and standard-brightness media

Desktop repair, 24 October 2026, on `codex/desktop-heart-preservation`.

## Finding and change

The installed album page for “To Dribble A Babangus” used a glaring coral window
fill. The Light artwork formula reached 100% of the native palette at only 15%
surface tint, then mixed toward its saturated raised-surface color. This is a
large-area color problem; the screenshot does not establish HDR display output.
The depth shader samples and displaces the original image without lighting,
bloom or exposure gain.

Light artwork surfaces now mix a bounded amount of their original palette into
neutral foundations. At the maximum 40% tint, the page uses at most 24% artwork
and raised surfaces at most 36%. Artwork-owned playback colors remain separate.
Dark, blue and explicit custom palettes keep their existing formulas. Saved
appearance preferences and the original artwork are not changed.

The document inherits `dynamic-range-limit: standard`, including portaled panels.
Existing artwork limits remain. Photo canvases and the depth compositor explicitly
use sRGB, retaining the existing 8-bit textures; WebGL already defaults to sRGB.
This makes the intended range explicit rather than claiming the previous renderer
was necessarily HDR. See [CSS Color HDR](https://www.w3.org/TR/2026/WD-css-color-hdr-1-20260728/)
and [Apple's standard-brightness guidance](https://developer.apple.com/videos/play/wwdc2025/233/?time=740).
No operating-system display preference is changed.

Home Play, collection Shuffle and shared Sketch buttons no longer apply brightness
multipliers on hover. Feedback uses the existing border/surface and press states;
fixed cover/control bounds, supplied glyph geometry and playback gradients remain.
There is no new frame loop or background task.

## Verification

Verification results and the signed delivery identity are recorded below.
Browser checks include saturated artwork,
contrast, inherited media range, hover bounds, sRGB compositor state, interruption,
late palette delivery, Reduced Motion and idle rendering. Theme assertions were
updated to the intentionally softer palette. One old transition fixture now opens
Albums explicitly because Home only shows a short shelf, and old collection
selectors now target the existing Home collection controls.

Native screenshots are saved under `out/qa/os-foundation/matte-sdr/`. Screenshots
verify appearance and layout, not physical HDR luminance. Human display acceptance
and Windows/Linux remain separate gates.

## Completed verification and delivery

- Production UI build and all 150 UI unit tests pass.
- All 29 focused browser tests pass, including 40 combinations of 720×480 /
  1280×800, all four interface scales, Light/Dark artwork and blue, and extreme
  custom color. Saturated artwork keeps at least 7:1 page-text contrast in the
  exercised maximum-tint fixtures.
- Artwork hover/scroll stays bounded; sRGB output/texture color spaces and inherited
  standard brightness are checked. Context loss recovers. Reduced Motion keeps
  ordinary covers. Idle layout notifications produce no extra draws or texture
  uploads, and preparation does not flash the cover. No native FPS claim is made.
- Menu closing/replacement, fixed Sketch controls, saved naming, collection color
  ownership, theme reversals, delayed palettes, Settings retention and dark first
  frames pass their existing regressions.
- Native release build and strict signature verification pass. All eight embedded
  runtime/model files match the previous installed app byte for byte.

The signed executable SHA-256 is
`56183e9cbeeac26b8a7104cc1f8ebe3869978ef309058aef056d05d17c24315d`.
Source commits: isolated desktop `0a75fbc`, primary mirror `289ed0c`.
The release record, full protected hashes and prior app are preserved under
`releases/desktop-matte-sdr-2026-10-24/`.

Installation followed a fresh paused UI check and clean quit, then a private copy
of the latest profile and a verified SQLite backup. Native verification opened the
same “To Dribble A Babangus” album: the intense coral fill is replaced by a soft
pink tint while original cover colors remain. Playback is still paused on “Girl
Like Me” at 2.0102084 seconds (0:02). The entire persisted playback snapshot matches,
including queue, index, volume, shuffle and repeat. All six collection tables,
favorite flags, interface preferences and integration configuration are unchanged;
the library contains 508 songs and 34 favorites. The app is left on that album.
