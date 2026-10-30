# Onboarding, product configuration and trackpad back — 2026-10-29

Work stays on `codex/desktop-heart-preservation`. The primary OS checkout is unchanged. Existing profile/data identities and original media remain stable.

## Delivered scope

- One shared `product.config.json` for identity, first-run appearance/scale/density, storage names and onboarding titles/version. Frontend, native desktop defaults and generated Android assets consume it. A validation/sync script updates package/Tauri/HTML display metadata; application IDs and old profile directory identities are deliberately stable. See `docs/product-configuration.md`.
- Fresh desktop profiles use per-user app-data media/database/sync paths, regardless of the working directory. Existing configured paths and environment overrides remain intact.
- Desktop welcome → optional name/appearance → music folders. Figtree, supplied Sketch icons, solid colorful surfaces and finite 180 ms content motion. Folder selection references originals; it is optional. Completion waits for acknowledged preference saving, with a visible retry on failure. Close/Escape does not write the draft. Existing profiles skip first-run setup; General settings can reopen it. No playback, sharing, metadata apply, model download or automatic scan is issued by the flow.
- Android has matching welcome, appearance and pairing guidance; explore-first is supported. Existing configured/populated/paired phones retain setup. Preference completion is acknowledged before dismissal. Short screens use smaller artwork and scrollable copy with stationary actions. Permission prompts remain on the actual pairing/scanning actions.
- Deliberate horizontal trackpad back gesture through Library navigation history: 110 px leftward wheel input, axis locking, a finite edge cue and 130 ms end-of-gesture commit. No page-scale/drag deformation. It reuses existing Back history/scroll/artwork return motion. Vertical/shift-wheel/pinch, horizontal shelves, sliders, editors, menus and queue panels retain input. Reversal below threshold cancels. New navigation, pointer input, blur, resize and unmount cancel pending work. Reduced Motion keeps a stationary cue and no spatial animation. Wheel events do not expose finger count; physical two-finger feel is a hardware acceptance gate.

## Verification

- UI: 168 unit tests passed, including optional naming, picker cancellation, failed persistence, deliberate gesture axes and exclusion of horizontal controls.
- Rust: 177 native app tests passed (2 explicit network tests ignored). Core storage tests passed, including fresh profile paths; existing relative/absolute path migration behavior is retained.
- WebKit: adaptive navigation 480/720/1280 and four scales; new-user completion/relaunch; no playback or scan calls; existing-user reopening/Escape without writes; welcome layout and fixed footer across Light/Dark and 80/100/125/150%; swipe once, vertical input, partial/reversed gesture, menus, Reduced Motion and cleared cue. New-user views were visually inspected after transition completion.
- Android: unit tests/lint/release signing passed. Emulator checks 360 × 480/720, Light/Dark, choices retained, failed-save retry and fixed actions. Existing Home/Library/Search navigation regression passed. Physical phone and trackpad remain separate acceptance checks.
- Existing mini-player geometry and photo rendering were retained in this pass.

The release manifest records signed hashes, unchanged embedded runtime/model hashes, the immediate pre-install private profile/SQLite checkpoint, and native post-load checks. Replacement is allowed only after a fresh paused/stopped check; current playback and newer listening activity must be preserved.
