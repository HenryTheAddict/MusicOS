# Collection controls bug fixes — October 15, 2026

Scope is bug fixes only. Your Radio, Hidden Gems and adaptive skipping are deferred in [the backlog](deferred-radio.md); no implementation started.

## Fixes

- Artwork picker grid now owns square cell geometry. Images are inset inside each cell and cannot keep an intrinsic height after grid rows shrink. Overflow scrolls without stacking photos on top of one another. Duplicate visible Cover artwork legend removed; accessible fieldset/header retained.
- Collection rows reserve independent artwork, name and action columns. Long names truncate within their column; Delete no longer wraps beneath the artwork in compact layouts. Confirmation and Smart rule editing remain intact.
- Smart field/comparison/matching and collection-sort dropdowns use the same themed, keyboard-operable menu. Menus portal out of page clipping, clamp to the viewport, expose selected state and restore trigger focus after Escape. For modal consumers, their portal stays in the dialog top layer. Original Smart definitions and validation are unchanged.

## Verification

- UI build and 70 frontend tests pass.
- Five browser tests pass, including artwork non-overlap at 720×480/1280×800 and 100%/150% scales, long-name collection geometry, keyboard menus at viewport edges, selected numeric comparison behavior, rule preservation, metadata Undo and 40 management size/scale/palette cases.
- Existing playlist creation/persistence and rule playlist favorite updates pass separately. Six grouped Settings/management checks also pass.
- Compact 150% artwork-picker screenshot, compact collection row and open Smart field menu inspected separately from geometry assertions.
- No new audio behavior, model weights, storage migrations or source-file operations are introduced. Prior audio/platform acceptance gates remain open.

Packaged build/native checks are recorded below after delivery.

## Delivered build

- Local ad-hoc signed Apple Silicon bundle: `target/release/bundle/macos/MusicOS.app`, 98.40 MiB.
- Executable SHA-256: `0cad258bde431a953e6bb802c8921c103f0e3ddbea44954d55752a2d1470f801`.
- Deep/strict signature verification passed. Embedded sidecar and ONNX runtime hashes remain unchanged.
- Loaded while paused and restored source position to 8.1 seconds. Library shows 477 songs, 11 favorites and the same saved collections.
- Packaged artwork picker opened through an unsaved new-playlist draft. Native screenshot confirms separated square cells, visible selected state and no overlapping photos. No playlist was saved and no artwork choice was applied by automation.
- User interaction repeatedly blocked subsequent close actions, so automated UI control stopped rather than clicking stale element indices. The user may still have the unsaved picker/draft open.
- Native Smart dropdown acceptance remains covered by browser keyboard/viewport fixtures; no user Smart definition was changed during package QA.
- Logs: `/tmp/musicos-collection-bugs-build.log`, `/tmp/musicos-collection-bugs-ui.log`, `/tmp/musicos-collection-bugs-final-browser.log`, `/tmp/musicos-collection-bugs-rules.log`, `/tmp/musicos-collection-bugs-package.log`.
