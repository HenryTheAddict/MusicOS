# Lyric glow, Sparklist creation and placeholder repair — 2026-11-03

## Changes

- Active word glow is an inert, inaccessible ink layer outside the lyric scroll mask. It copies exact active-line typography/wrapping; automatic scrolling updates its transform in the same frame. Scroll/layout/font changes update measured geometry. It has no independent playback frame loop and releases its nodes, listeners, observer and queued measure on closure/inactivity. Reduced Motion removes it.
- Foreground lyric text uses Figtree at a stable 600 weight with less compressed tracking. Heavy outlines and stacked foreground glow are removed. Visualizer text retains a restrained contrast shadow. The original line owns punctuation, whitespace, Unicode and syllable adjacency; incompatible provider labels fall back to line presentation instead of rewriting the displayed text.
- Word state includes explicit end boundaries, including vocal gaps, overlapping words and backward seeks. Genuine word ends, next starts or explicit line ends drive progressive fill. No fabricated final-word duration is used. Existing acknowledged clock, offsets and duet grouping remain authoritative.
- Sparklist creation uses stronger theme-compatible gradients, direct stage labels, and fewer redundant status lines. One bounded review list scrolls between stationary identification and actions. Short windows use a compact summary. Hidden removal labels are contained within their buttons rather than extending the outer dialog's scroll area; border/padding accounting avoids an extra outer scroll range. Selection and naming workspaces scroll only when needed.
- Soft padded edge fades indicate actual remaining content in Sparklist selection, review and naming. First/last rows are fully readable at the ends. No always-running scroll animation is added.
- Promotional taglines are removed from Home, collection management, About and empty Now Playing; Devices and onboarding use direct functional wording. Preview listening and saving remain explicit actions.
- Shuffle playback actions retain supplied Sketch strokes/play geometry while excluding the white export canvas. Existing ON/OFF morph and playback actions remain intact. Missing artist images similarly exclude the white canvas, using the supplied fallback drawing inside a full-size themed portrait silhouette. Original icon source geometry is unchanged.

## Verification

- UI production build succeeds. 235 UI tests across 74 files pass, including exact line text, word-end gaps, overlapping word timing, backward seeking, missing ends, inert glow ownership/cleanup and no independent frame loop. Existing Shuffle endpoint/reversal tests pass; canvas removal and portrait geometry are covered.
- 39 browser checks cover the player and Sparklist creator at 480×480, 720×480 and 1280×800, 80/100/125/150% interface scales, Light/Dark artwork, blue and extreme custom surfaces. Checks include fill/pause, end-of-word gaps, RTL wrapping and glow alignment, Reduced Motion cleanup, shared player coordinates, bounded review scrolling, padded end fades, visible actions, explicit save without playback, transparent Shuffle glyphs and missing portraits.
- Browser screenshots inspected for the gradient creator, artist grid, transparent menu glyph, and RTL lyric treatment. The isolated native WebKit/Metal fixture loaded timed word lyrics and showed the active word over the native visualizer after returning from fullscreen. This fixture is silent and never loads the user's profile. It does not establish audible provider-edition timing accuracy.
- The native test fixture URL now explicitly selects genuine word-timing fixtures. No production playback command, audio callback, Android schema, Wi-Fi manifest, database or saved preference changes are made.
- New glow work is bounded and does no continuous layout reads; paused ticks do not schedule a glow loop. No new frame-pacing/audio benchmark or physical gesture acceptance is claimed. Existing broader Automix/Apple/mini-player acceptance limitations remain unchanged.

## Delivery

Deliver from the isolated desktop branch. Preserve the latest profile/collections and previous application. Compare all eight protected embedded runtime/model hashes with the installed build, record the signed executable hash, and install only after a fresh paused/stopped native check. Leave running playback untouched.
