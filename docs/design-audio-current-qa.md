# MusicOS design / Audio implementation verification — 2026-10-10

This effort is still in progress. The release gates in the approved plan are not satisfied by browser checks.

## Current corrections

- Album grid captions cap at two lines, contributors at one; full strings remain in tooltips and details.
- Hover cards expose the supplied transparent Play SVG. Card overflow is available through right click, without a visible three-dot control; Shift+F10 opens its menu and Escape restores focus. Detail Play has no pill, background, border or inner shadow.
- Favorite motion is now a single 1.10 scale pulse and fill transition, without tilt or particles. Rapid toggles cancel earlier animations; reduced motion suppresses the pulse.
- Song artwork checks each file's embedded front cover before using album artwork. A desktop-only identity cache retains positive and negative results; changes invalidate them. File metadata is not rewritten. Song artwork updates do not replace the album's selected cover.
- Track quality, favorites and overflow share a single grid action area, preventing the menu from wrapping onto a second row.
- Audio replaces Queue, with docked/floating preference and compact-window fallback. Analysis management is in Settings > Audio. Queue occurrence IDs, pointer reordering, keyboard reordering and pinning are implemented.

- Artist profile updates no longer refresh the entire library or invalidate unrelated cover images. Missing portraits use a sharp centered placeholder and a small loading indicator. Loaded portraits stay visible during background revalidation. Completed cached images do not restart the loader on rerender.

## Checks

- Frontend unit checks: 28 passed, including song-specific artwork preference, unreadable artwork fallback and missing-artwork caching.
- Desktop metadata checks: embedded red/blue covers in two real FLAC fixtures remain distinct, extraction preserves file bytes, and removing embedded artwork invalidates cached results. Codec probes distinguish FLAC, ALAC, AAC and MP3.
- Browser focused checks: transparent hover/detail Play, hidden card dots, two-line captions, rapid favorite toggles, reduced motion, portrait loading, and cached-portrait stability during playback passed.
- Latest complete browser run: 55 passed; the final test failed during trace cleanup with a missing artifact after overlapping test runs used the same output directory. The nine interaction/compact-Audio checks then passed in a serial rerun. Earlier full suite: 55 passed before the additional card-control regression.
- Tauri checks: 58 passed, including the new embedded-artwork regression.
- Desktop local service checks: 23 passed, including actual decoder handoff and source accounting checks. These are not listening evaluation.

## Native and model gates still open

The current Rows QA package has been opened and inspected natively. Song quality,
favorite and overflow actions are aligned in the album view. This observation
does not establish acceptance of all native behavior or musical transitions.
Do not replace or terminate a playing app without an approved restart.

Real-recording annotated analysis benchmarks, subjective transition listening, packaged pitch/clipping/source-time acceptance and Sparklist model listening/hosting gates remain open. The Sparklist model is not production-approved; generation remains visibly unavailable. Creative brake/acceleration effects and production model delivery are not complete. Windows/Linux are unverified. Lyrics and stems remain deferred; the embedded runtime is unchanged by this work.

## Corrected macOS package

Final build completed with Tauri 2 on Apple Silicon. The staged, ad-hoc signed
`out/rebuild-qa/MusicOS Design QA.app` passed `codesign --verify --deep --strict`.
Executable SHA-256:
`9f5a1207e99175416a2adba80878f7765c158996eb81c6ddd93c8a6ad648de46`.

Final checks: 28 frontend unit checks, 58 Tauri checks, eight focused interaction
checks (including the missing-portrait/revalidation regression), and a separate
transparent Play/keyboard-menu check passed. The earlier 23 local playback checks
remain applicable. The corrected native build is staged; restart approval is pending.
The running app and playback have not been stopped.

## Song action-column follow-up

Local song tables now use explicit named grid areas for number, title, album,
time and one action group. Compact layouts and album details each declare their
own complete grid, so the action group cannot fall into an implicit second row.
The header uses the same five-column schema. Quality, favorite and overflow
remain aligned and the overflow menu is functional.

Play now replaces the number on hover/focus, keeping the title's full width and
position. All Play glyphs and song overflow controls have transparent backgrounds,
no inner shadow and no oval container. A 24-case browser regression passed for
Songs, album and playlist routes at 720×480 / 1280×800 and 80/100/125/150%.
The user located the remaining unwanted oval in album/playlist view; the precise
element (header controls, song background, or artwork) is still awaiting
clarification. No blanket rounding change has been made.

## Latest native follow-up build

`out/rebuild-qa/MusicOS Rows QA.app` supersedes the Design QA package above.
The Apple Silicon package built successfully and passed strict deep code-signature
verification. Executable SHA-256:
`4aca18d6db03de8cbe2eeccd1ed54939e957dc43cb6c1dd623121c0041e74bb0`.

Native inspection at 1280×800 logical pixels confirmed the album's quality,
favorite and overflow controls share one line, and the header Play glyph has
no surrounding oval or inner shadow. Playback controls were not changed during
this inspection. Five focused browser tests passed, including the 24 route/size/
scale combinations, stable title placement, keyboard access, rapid favorite
toggles and reduced motion. The requested remaining oval is not marked resolved.

## Compact credits and Sparklist follow-up

Artwork cards no longer expose a corner Play button. Right-click/keyboard
menus and detail Play remain available. Albums with more than two contributors
use “Various artists” and small clickable portraits, ordered with the primary
artist first. Hover and keyboard focus reveal names without expanding the header.
Descriptions use the available width, preview up to five lines, and offer
Read more/less only when text actually overflows.

Sparklist creation retains the selected seed while filtering, prevents generation
from unanalyzed seeds, offers Audio settings, reports preview duration/coverage,
and plays a preview only on an explicit Listen action. Generation and feedback
alone do not replace the queue. The production model gate remains closed; see the
mood-discrimination failure in `docs/sparklist-model-evaluation.md`.

Checks: 32 frontend unit tests and 10 focused browser tests passed. Browser checks
include 18 contributors at both target sizes and all four interface scales,
hover/focus navigation, description expansion, artwork-corner removal, and the
24 song-action layout combinations. Native acceptance of this follow-up remains
pending; the running app has not been restarted.

The follow-up Apple Silicon package is staged as
`out/rebuild-qa/MusicOS Credits QA.app`, ad-hoc signed and verified with strict
deep signature checks. Executable SHA-256:
`1eace3754e12e5fa9e1662782d4b9eca648d9061de4619466e7e413b28e0b99d`.
It has not replaced or restarted the running app.

## Adaptive artist hover labels and launch

Artist hover/focus labels now use an accent-tinted theme surface, palette text,
a subtle palette edge and a tinted shadow rather than an inverted dark label.
The eight size/scale contributor cases passed again. The Apple Silicon package
built and passed strict deep signature verification after ad-hoc signing.
Executable SHA-256:
`9f6bc40a9813d5c6ba95a0f6da44e583d06cdada0d63a321c530ac46105219dd`.
The user requested launch; this build is running from
`target/release/bundle/macos/MusicOS.app`. Native inspection confirmed the UTOPIA
view has compact, separately clickable contributors. This does not establish
Sparklist model or DJmode native acceptance.

## 2026-10-10 — direct queue dragging and complete album credits

- Album header retains the primary artist's visible name and portrait; collaborators are small circles with adaptive name pills. The generic “Various artists” replacement is removed.
- File metadata cache identity advances to v2 and reads every ARTIST value and semicolon-delimited collaborator. Explicit user artist overrides remain authoritative. Files are read without rewriting tags. In the configured native library, UTOPIA now caches 18 distinct credits rather than three.
- Audio rows use direct pointer dragging from artwork/text, with a movement threshold, no visible drag handle, insertion markers, edge autoscroll, and Escape cancellation. Capture belongs to the stable scrolling container so virtualization cannot cancel a drag when its original row unmounts. Option+arrow keyboard moves remain.
- Up next excludes current/past entries and translates visible indices back to authoritative absolute queue indices. Artwork uses per-track extraction, matching the player. Compact rows bound titles to two lines and keep durations/removal aligned.
- Checks: 59 Rust library tests passed; 32 frontend unit tests passed. Contributor layout checks passed at 720×480 and 1280×800, all four scales. Queue browser checks cover direct drag, keyboard moves, duplicate occurrence removal, current-track preservation, 10,000-entry virtualization, autoscroll, Escape, and file-import isolation. Focused playlist editing/restoration checks passed.
- Native packaged app rebuilt and ad-hoc signature verified. Executable SHA256: `ca2d1c0e98d87693dd4156a33acc19ec6aefacb40a4e402f31459e50d08c7df2`. Restarted only after confirming nothing was playing; loaded the configured 393-song library. The user began interacting with the native app, so further automated native navigation was stopped. Native drag/listening acceptance remains unverified for this build. Sparklist model/listening/hosting gates remain unchanged.
