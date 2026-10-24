# Mini-player size, dragging and context motion — 2026-10-24

Compact now has a 220×120 minimum; Wide is 320×120 with a 48 px cover. Both share the same centered control row, 8 px outer padding and 4 px frame gap. Standard remains 244×320. The compact queue omits a redundant visible heading so its current-song header and a complete upcoming row fit without moving the footer. The Up next region keeps its accessible name. Native minimums, pointer/keyboard resizing, menu-space fallback and still Appearance previews use the same dimensions. Existing preset preferences and per-session resized bounds remain supported.

Long overflow titles remain keyboard focusable and can initiate native window dragging. Controls, seek bars, editable text and queue dragging retain their own input. During acknowledged native menu expansion/collapse, the frozen player-height marker prevents window dragging from racing the operation. No grab handle or new playback/database contract is added.

Action menus use a finite 140 ms opacity/scale reveal from their pointer or button attachment, followed by a 90 ms dismissal. Dismissal captures the displayed opacity and transform, including interrupted entrances. Action selection still removes the menu synchronously before executing the action, so it cannot swallow input from a new dialog. Mini-player actions use corresponding motion while retaining exit-before-window-collapse ordering. Reduced Motion disables menu shape animation; a conflicting selector that previously overrode this preference is corrected.

## Checks

- Production UI build, 138 UI unit checks and 155 Rust library checks passed.
- All 24 minimum-size combinations passed: three presets, four interface scales, Light/Dark. Artwork/queue footprint and a complete upcoming row were checked.
- Focused browser regressions passed for context-pointer origin, intermediate transform/opacity, interrupted dismissal, Reduced Motion, keyboard/focus restoration, page interaction, all three creation dialogs, queue reversals/identity, transparent corners, Play/Pause endpoints and finite mini-player entrance/exit.
- Drag checks cover nested text, focusable long labels, controls, queue gestures, failed native requests and the entire menu-resize interval.
- Evidence remains in ignored `out/rebuild-qa/`, with native delivery recorded below after loading.

This pass does not claim measured native display pacing or guaranteed frame rate. Physical touch and Windows/Linux native behavior remain separate gates. Embedded runtime and model resources are preserved.
