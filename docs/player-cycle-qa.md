# One cycling player control — 1 November 2026

The expanded player, standalone listening panel, and all mini-player presets now use the same order: normal → Up next → Lyrics → normal. Top player tab strips are removed. The button displays the next action, with an accessible name and tooltip. It supports pointer, keyboard, existing hidden-chrome behavior, and interaction haptics.

The queue outline, lyric strokes, and supplied Song icon use compatible authored cubic curves. Exact algebraic subdivision preserves Sketch curve geometry; no sampled contours or opacity-only icon swap is used. The 180 ms morph starts from its displayed geometry when interrupted. Reduced Motion switches directly, hiding cancels temporary work, and completed morphs leave no frame callback.

Transport and seek controls remain stationary. Expanded content uses the existing finite transition owner; hidden outgoing content is inert. Standalone queue and lyrics retain visited content, queue identity and scroll. Mini-player presentation changes do not invoke native window-mode/resize operations; the existing artwork/queue preference remains available. Removing its header row gives artwork and lyrics additional room within unchanged preset bounds.

Validation:

- UI build succeeds; 203 unit tests pass, including three-state order, accessible action labels, exact authored path endpoints, intermediate geometry, interruption/reversal, Reduced Motion, and no remaining morph frames.
- Browser checks cover cycling and retained queues, no presentation-issued playback commands, all mini-player minimums, fixed transport/control rectangles, hidden chrome, native titlebar alignment, fullscreen and menu/resize behavior.
- The size/theme matrix covers 480×480, 720×480, 1280×800, 80/100/125/150% scales, artwork Light/Dark, blue, and extreme custom colors. 38 cases completed; one matrix case timed out in the combined run and passed in an isolated rerun. Existing cancelled dynamic-module warnings appear during test page reloads.
- No native playback logic, audio protocol, Android schema, Wi-Fi manifest, embedded runtime, or model change. Delivery requires a fresh paused/stopped check, signed hash and protected-file comparison, latest profile backup, previous app archive, and native post-load verification.

The user confirmed that the preceding release's Force Touch beat pulses and interaction ticks both feel right. That acceptance is recorded in its release manifest.

Installed after freshly confirming Mercury paused at 83.9 seconds. It restored paused at the same position. The native Queue control exposed its Lyrics next action; the user then resumed playback and navigated during further inspection, so that activity was left untouched. Signed executable SHA-256: `3582945fb4fb4cf78ccd8f794cf36c4a8757f199dd5fcdc9b7cd49bb14690abb`. All eight protected runtime/model hashes are unchanged. The profile comparison permits only serialization of the already-enabled default trackpad settings; existing settings, collections, lyric revisions, and newer listening activity are preserved.
