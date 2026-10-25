# Mini-player settings and artist portrait declutter — 2026-10-25

The size picker now presents a compact Standard / Compact / Wide selector and
one inert preview of the selected size. Dimensions and the size description are
shown once. Artwork / Up next remains a separate preview choice. The preview
reuses the actual mini-player presentation and its existing density metrics;
window sizes, playback commands, metadata requests and preferences are unchanged.

Artist photos no longer produce floating genre, year, album or song badges on
hover or focus. The existing library statistics remain together below the photo.
Photo attribution is a separate, initially collapsed Photo credit disclosure
below the playback controls, retaining photographer, license and source action.
The photo still opens the artwork viewer and regains focus after Escape. The
full biography and its expansion, colorful Light artwork palette and SDR photo
pipeline are untouched. Obsolete badge and three-card layout rules were removed.

## Verification

Production UI compilation and all 154 UI unit tests pass. Nine distinct focused
browser cases pass across the final and regression runs. These cover selection
and persistence, a single inert preview, no playback or metadata commands from
preview interaction, both preview views, clean portrait hover/focus, separate
keyboard-accessible attribution, artwork viewer focus restoration, retained Audio
settings, biography expansion/collapse, interruption and Reduced Motion.

The artist layout/contrast matrix passes 40 window/scale/theme combinations.
The picker matrix passes 48 combinations, checking all three presets in each:
720×480 and 1280×800; 80%, 100%, 125%, 150%; Light/Dark blue, Light/Dark artwork,
and extreme Light/Dark custom colors. Neither picker nor Settings panel acquires
horizontal overflow. Actual minimum dimensions remain 244×320, 220×120 and
320×120. Finite preview animations settle without leftover animation work.

The first portrait check exposed an older identity-transform rule; the still
portrait owner now overrides it explicitly. A fixture credit test initially
injected against a source comma removed by the dev transform; its injection was
corrected and the complete attribution/focus check passes.

Browser screenshots are under `out/persistent-qa/` in the isolated desktop
worktree. Native build, installation, preservation and screenshots follow below.
