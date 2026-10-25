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

Verification results and the signed delivery identity are recorded below after
the native candidate has been checked. Browser checks include saturated artwork,
contrast, inherited media range, hover bounds, sRGB compositor state, interruption,
late palette delivery, Reduced Motion and idle rendering. Theme assertions were
updated to the intentionally softer palette. One old transition fixture now opens
Albums explicitly because Home only shows a short shelf, and old collection
selectors now target the existing Home collection controls.

Native screenshots are saved under `out/qa/os-foundation/matte-sdr/`. Screenshots
verify appearance and layout, not physical HDR luminance. Human display acceptance
and Windows/Linux remain separate gates.
