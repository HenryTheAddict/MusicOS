# Window, panel, theme, and motion repairs — 2026-10-17

- Window dragging now starts from mouse-down on noninteractive header/text backgrounds. Descendant text participates, while buttons, seeking, and queue rows retain their own behavior. Failed native drag requests show an error instead of being discarded.
- Pin window / Unpin window controls native always-on-top behavior. Mini-player pin state survives layout changes and temporary menu expansion. Returning to the main window restores its previous native bounds and always-on-top state.
- Fixed the artwork mini-player menu inheriting `pointer-events:none`. Its menu has rounded clipping, a bounded scroll area, and keyboard focus restoration. Native expansion clamps the intended physical window size to the display work area instead of reading stale bounds during queued resize.
- Panels and specification popovers use rounded clipping through resize. Up next and Audio share a header, close-button lane, and current-song row. Queue opening uses a short opacity/translation animation, with no scaling and no animation under Reduced Motion.
- Removed the fresh-depth opacity animation. Shared depth caches and GPU recovery remain intact.
- Shuffle changes retain their supplied drawings and use a 100 ms opacity fade rather than contour morphing. Play/pause and favorite icons use contrast-checked theme ink; the circled menu face follows the control surface. Supplied Pin geometry was exported from Sketch (`67796EA7-CF22-4970-8CC3-AC15F64B7B66`) and added to the asset manifest.
- Library album titles use weight 750 and artist captions 400. Text remains left aligned; surfaces remain solid.

## Verification

UI build passed. 82 unit tests and 132 native Rust tests passed. Browser suites exercised 33 distinct checks across mini-player, panels, depth, deep scrolling, touch controls, and audio controls, including compact/wide windows, all interface scales, and five palettes. Final targeted rerun covered 16 checks. Fresh depth-map preparation was observed without any artwork opacity animation. Theme changes were checked on the rendered SVG path, not just its parent color. Pin state, menu bounds, keyboard access, and Reduced Motion passed.

Screenshot review included the bounded mini-player menu, aligned queue panel, and compact dark Audio panel. Automated queue edits used disposable browser fixtures. Native delivery results and signed build identity follow below.

Loaded final ad-hoc signed macOS build, executable SHA-256 `cfe98c2c46342fb4d7f16813d266b9d2f9f50a50e2e64d5bbf044408533ee15c`. Deep strict signature verification passed. Embedded playback and ONNX runtime hashes remain unchanged (`6b1fa66d…` and `ff52828c…`).

Native checks confirmed Unpin → Pin, retention in the artwork layout, and Pin from that layout; actions completed without errors. The final menu screenshot confirmed rounded bottom corners, scrolling rather than window-edge clipping, and the transparent supplied Pin icon. A title-area native drag gesture completed without an application error; exact displacement was not measured. The final reload changed only the Pin export background after those interaction checks.

Left the final build in the Dark queue mini-player with its menu closed, pinned, and playback paused at 122.7 seconds on The Prime Time of Your Life. The eight upcoming entries, 507-song library, and 14 favorites remained intact. No real queue membership edits were performed during native QA.

Windows/Linux, cross-monitor/fullscreen restoration, quantified drag displacement, 120 Hz frame timing, and human audio listening remain separate acceptance gates. No audio DSP or source-file changes are included.
