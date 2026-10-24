# Expressive desktop controls — October 24, 2026

## Delivered source behavior

Song-row Play is back in the title column: numbers stay visible, labels move
1.75 rem in 180 ms, and hovering/focusing a playing song morphs its meter into
Pause. Controls retain click isolation, row selection, double-click playback,
context menus, and pointer capture only after a 6 px drag threshold. The glyph
stays mounted when acknowledged playback changes. Meter subscriptions are limited
to the playing row and do not restart the shape animation.

The meter/action morph uses the authored Sketch transport contours and compatible
rounded meter bars. Supported engines animate SVG `d` through Web Animations;
interruptions sample the displayed timing, with a frame-driven fallback for older
engines. Reduced Motion switches immediately. Main Play/Pause remains 360 ms.

Drag previews follow actual pointer coordinates with a separate soft deformation
layer, bounded to 3% and 3 degrees. Drawing is coalesced to a frame, dimensions are
observed rather than read on every move, and idle deformation stops within 600 ms.
Release presentation lasts at most 180 ms; cancellation removes it immediately.
Existing accepted/rejected drop logic, duplicate handling, order and scroll remain.

Home puts collections directly after Continue listening. Resume stays explicit;
mounting Home never starts playback. Inset effects are removed from the shared
styles while outer shadows/focus indicators remain. The connected section arrow
has one owner: down closed, up open. Settings uses role-specific choices/actions,
themed sliders/progress, coherent selectable mini-player previews, and full-width
metadata browser rows. Preset dimensions and playback commands are unchanged.

## Verification

- Production UI build and macOS release packaging pass.
- 145 UI unit tests and 155 Rust library tests pass.
- Focused interaction tests cover slide/meter reversal, keyboard activation,
  bounded jelly, idle cleanup, retained numbering, precise drops, Home's explicit
  Resume, connected chevron geometry and Reduced Motion.
- Window/theme matrix: 720×480 and 1280×800, 80/100/125/150%, Light/Dark/blue/
  artwork/extreme custom. Selected-control contrast is at least 4.5:1; metadata
  actions stay within their rows and inspected surfaces have no inset shadows.
- Existing navigation, context menu replacement, multi-song drops, duplicate-only
  drops, block reorder, queue additions, edge scrolling, mini-player hearts,
  metadata review and hidden-page privacy/polling checks are retained.
- Legacy test setup now opens creation through the current creation menu. Persistent
  Settings tests address the real mini-player, excluding inert previews, and wait
  for its entrance before comparing stationary control geometry.

Evidence lives in `out/rebuild-qa/expressive-*`, with packaged/native delivery
records under `releases/desktop-expressive-controls-2026-10-24/` in the primary
checkout. Private library/profile snapshots and model results stay outside Git.

## Performance limits

The same automated Chromium fixture was compared against source at `f1f7caa`.
Five-second hover trials paced at roughly 16.7 ms median in both versions, with
p95 around 16.8 ms before and 16.7 ms after. Outliers varied between trials;
this is not sufficient evidence of a reproducible FPS gain.

The same-page morph microbenchmark removes 3,000 computed-style reads over 250
morphs, uses no per-frame JavaScript for native path animation, and has setup/total
script times in the same small range as the previous implementation. It does not
establish a meaningful total scripting-speed improvement. Restoring visible motion
adds work compared with disabling it entirely; this tradeoff is recorded rather
than claiming a guaranteed frame rate.

Native appearance and paused state are inspected separately. Physical-display frame
pacing, subjective motion acceptance, physical touch and Windows/Linux remain
acceptance gates; browser fixture measurements are not native results.

## Package integrity

Signed executable SHA-256:
`c7860261259f447e3dedad12762c732f9922ddff55d773b40e48584a29e68861`.

Strict deep signature verification passes. Compared with the installed baseline,
all five protected resources match: `musicos-mac`, ONNX Runtime, `audio.onnx`,
`moods.json`, and `preprocessor_config.json`. Full hashes are in the release
manifest. The previous app is retained. Installation requires a fresh native paused
check, a clean quit, a verified SQLite backup and a copy of the latest profile.
