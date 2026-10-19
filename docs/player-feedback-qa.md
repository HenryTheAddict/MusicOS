> Superseded by [mini-player-frame-qa.md](mini-player-frame-qa.md) after the user reported blocked mini-player controls and rejected native popup styling. Intermediate results below are historical.

# Player and dark-theme feedback — 2026-10-17

This pass restores left-aligned Now Playing and collection captions, removes page/grid entrance and elastic control motion, and keeps the requested gentle depth effects and initial launch color fade. Theme changes after launch apply immediately instead of fading text and backgrounds through low-contrast intermediate colors.

Artist captions previously inherited hard-coded black from the Sketch stylesheet. Captions now use theme ink; dark secondary text is #c4c8d0. Dark surfaces stay #202124 and #2e3035 for all artwork/custom accent choices. Higher-specificity dark sidebar rules replace the light adaptive surfaces that were still winning the cascade.

Mini Player keeps its two sizes and authoritative playback state. Tall mode uses one transport row with favorite and actions at its sides, leaving more room for square artwork. All copy is left aligned. The invisible whole-player drag overlay is removed; the song/credit text is the drag region, with no added handle. On macOS standard window buttons stay hidden in Mini Player and are restored on exit. Hiding their parent container froze the first packaged handoff, so that change was removed. The native window/webview background is transparent in Mini Player so CSS rounding does not sit over an opaque rectangle.

Now Playing and Mini Player use native popup menus in desktop playback. A menu rendered inside a 48-pixel webview cannot extend outside that window; native popup menus can. Menus are replaced/disposed between opens, use current action handlers, and expose failed opening. A pending native popup promise no longer holds the opening guard; the regression test verifies another open remains possible. Browser fixtures retain the web fallback. Mini Player's Up next action and direct queue button first restore the main window, then open the existing queue. Clicking the slim artwork expands it directly; the tall artwork opens the album. Escape dismissal now restores focus to the queue button even when WebKit does not focus a pointer-clicked button.

## Automated checks

- 78 UI unit tests passed, including native popup delivery/action/resource disposal/failure and neutral dark palette checks.
- 16 WebKit player/queue checks passed: links/alignment, right-click actions, both mini layouts, Mini Player → Up next → song selection, dark sidebar/artist colors, queue duplicates, drag/reorder, insertion, dismissal, and four compact scales.
- The first queue run caught focus restoration using document.activeElement (body in WebKit). This failed result was retained and the opener fallback repaired; the full suite then passed.
- 132 native library tests passed; production TypeScript/Vite and active Tauri checks passed.
- Final layout/deep-scroll regression and packaged native checks are appended below.

No file tags, source files, favorites, or collection membership are changed by native QA. Embedded playback and ONNX runtime binaries are preserved. Windows/Linux and audible listening acceptance remain unverified.

Final automated layout/deep-scroll checks passed 16/16 in WebKit. After adding the direct miniature controls, all seven player cases passed again, including cover expansion and direct Up next opening at compact sizes. Native popup opening and the rounded slim window were inspected in the packaged app; automated selection of popup items outside its 48-pixel window was unreliable and is not claimed as a packaged selection pass.
