# Artist page layout repair

25 October 2026. Desktop follow-up to the two Travis Scott screenshots.

The previous layout gave the artist name most of the header, put a pill inside
an About card, and spread portrait, facts and playback over separate grid cells.
Expanded biography height stretched a row shared with the facts, pushing playback
far below the portrait. That was a layout error rather than intentional spacing.

The artist header now has one stylesheet, replacing the superseded artist rules
across the legacy Sketch/refinement layers. The name is bounded and left aligned.
About is plain readable text, with a four-line preview and Read more / Read less
beside its heading. Full paragraphs and source attribution remain available.
The portrait, library counts, duration and supplied playback controls form one
independent summary. They remain together at the top during expansion. Below
48 rem of available artist width, the summary becomes a compact thumbnail/facts/
controls group above About. Albums follow the compact header without the previous
oversized title and card spacing. Photographs retain the SDR display path and
original files are untouched.

Expansion animates the measured containing height over 180 ms without scaling or
reflowing text into different column widths. Reversals start at the displayed
height. Scrolling, resizing, Reduced Motion and unmounting release the temporary
animation. Navigation animates the summary as one group rather than animating its
statistics separately. The 320 ms visible-cover transfer remains intact.

## Verification

Browser checks exercise the long biography in both sizes (720×480 and 1280×800),
all four interface scales, Light/Dark blue, Light/Dark artwork and an extreme dark
custom color: 40 combinations. Assertions cover square portrait bounds, page
bounds, readable biography contrast, stable summary spacing and complete expanded
paragraphs. Separate checks cover keyboard activation/focus, rapid reversal,
resize, Reduced Motion, no leftover animations, original photo interaction,
artwork transfer and return, and no playback commands from biography controls.

Existing tests were updated for the summary's semantic grouping, current supplied
SVG gradients and the existing simultaneous 180 ms entrances. The artist-player
helper now explicitly opens Albums before finding Junk, and its queue assertion
uses the retained track crossfade wrapper. Geometry assertions wait for the finite
biography height animation before checking the album section position.

Build identity, final test counts and native delivery verification are recorded
below after completion. This change adds no database schema, playback protocol,
preferences or model processing. Physical display pacing and human motion
acceptance are not inferred from browser screenshots.
