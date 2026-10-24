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

## Native delivery

Source `a91f410`, copied to the shared foundation as `61e7bc5`; the primary compatibility check passed and retained `connected_device`. Signed executable SHA-256: `88d318bc3d33296a68c1a51574f834cfcad82bc3102ecb03f2a824ea211f4384`. Ad-hoc signing and strict deep verification passed. The embedded runtime, ONNX library and all three model resources match the preceding release exactly; the full manifest and previous application are retained in `releases/desktop-mini-refinement-2026-10-24/`.

A fresh native paused-state check preceded clean quit, private profile copying and a verified SQLite backup. BUFFALO resumed at 24.7 seconds paused with 508 songs. All six saved collection/artwork/membership tables compared exactly with the immediate pre-load backup; integration configuration bytes were unchanged. No profile snapshot was restored.

The native Wide capture is 640×240 pixels at 2× density, matching 320×120 logical units. A native drag gesture completed without error or playback-position/transport changes. Right-click opened mini-player actions; Escape restored the original window footprint and focus. Switching artwork/queue retained the same window size and bottom controls, with one complete upcoming song visible. The artwork view and Wide preference were retained after checks. macOS cursor/window-sharing overlays visible in the native capture are separate from MusicOS artwork. Native evidence and checks are under `out/qa/os-foundation/mini-refinement/`.
