# Sketch Library and adaptive desktop shell

The desktop switches below 56 rem of effective width, using one scale-aware
layout observer. Its persistent Now Playing node moves between grid slots;
resizing does not remount transport controls or send playback commands. The
normal native minimum is 480×480, including restoration from every Mini Player
preset. Existing preset sizes, saved bounds and wide artwork preferences remain.

The compact shell uses Home / Search / Library tabs, a separate compact player
above them, and full-width content. Each destination retains its detail stack
and scroll, including identical albums opened through different destinations.
Search reuses SuperCommand's results/actions inside a retained page. The wide
command palette remains. An in-window modal player provides artwork, transport,
seek, volume, shuffle/repeat, song actions and the existing identity-aware queue.
It remains open through resizing without changing the native window size.

Library.svg is the supplied Sketch master
E4FDE931-A4E1-4E34-B419-3560D9BBEB8C from musicosdesk.sketch. Its three overlapping
32×32 faces and two-unit strokes are preserved; only the export's full-frame
white canvas is removed. Faces take the active surface color. Home and branding
keep their separate assets. The Long Miniplayer and Tab references informed the
compact arrangement, Figtree, rounded solid fills and supplied glyph sizes.

More now keeps its label, button footprint and normalized icon slot fixed through
hover/press/open/close. Only the supplied dots/close glyph morphs. Dialog menus
portal into their owning modal, so they remain above and interactive within it.
An outgoing inert menu cannot consume the next Escape in a new dialog. Queued
scroll events from revealing a trigger do not immediately dismiss its newly
opened menu; subsequent scrolling that moves the attachment does. Existing
140 ms reversal, connected seams, shadow and session focus guards remain.

Compact Settings categories wrap legibly. Underlying player controls become inert
through the native modal dialog. Reduced Motion skips spatial/shape animation;
transfer clones and finite animations are removed after completion or resizing.
Heart layers share their centered hit box; albums without descriptions release
the empty description column and reduce the controls' top spacing.

## Evidence

The production UI compiles and 161 unit tests pass. Focused browser coverage
includes independent navigation/Search, persistent player identity, duplicate
queue entries, keyboard queue reordering, empty playback/queues, modal/nested
Escape and focus restoration, pointer Space, delayed artwork, all interface
scales, retained Settings drafts, 480 px metadata/creation, Reduced Motion and
repeated resizing. The layout matrix covers 480×480, 720×480 and 1280×800 at
80/100/125/150% with Light/Dark artwork, blue and extreme custom colors. Existing
metadata, mini-player preview and selector intermediate-frame regressions are
also exercised. Final engine counts are recorded in the delivery record.

A same-machine Chromium fixture comparison against installed UI source 512d2a0
used identical wide-shell menu/navigation actions. Median frame interval was
16.7 ms and p95 was 16.8 ms in both runs; both had five intervals over 1.5× median.
Layout work was 25.4 → 23.9 ms; measured frame callback work 7.2 → 6.7 ms. Total
script work was 369 → 387 ms and task work 794 → 826 ms, small single-run variation.
Both left zero active animations. This is browser automation, not a measurement
of the native display or a promise of higher FPS. Native pacing and physical
input acceptance remain separate from these results.

No existing preferences, schema, playback protocol, SDR correction, installed
models/embeddings or OS code changes in this desktop pass. The app/profile are
preserved; load remains gated on a fresh paused/stopped check. Android/Wi-Fi work
requested subsequently is delivered separately from this commit.
