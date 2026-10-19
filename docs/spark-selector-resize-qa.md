# Sparklist picker, hover fading, and resize colors — 2026-10-14

## Changes

- Select song opens a dedicated modal picker with artwork, credits, durations, and optional filtering. Selection closes it; Change song reopens it with the selection visible. Escape closes only the picker and restores focus to the trigger.
- The picker virtualizes large libraries. Measurements taken while closed cannot cache zero-height rows. Resize measurements run through an animation frame.
- Revealed song titles and credits fade at the right edge instead of ending in a hard clip. Other song columns and the play reveal retain their geometry.
- The page frame has a palette-matched solid backing beneath the animated theme layers. The shell token is also propagated to root-level surfaces. Resize does not trigger palette lookup or theme entrance animation.

## Verification

- UI build passed; 63 frontend tests passed.
- All eight collection browser cases passed: creation, selected-song preservation, nested Escape/focus, preview playback, explicit save, compact sizing at four interface scales, and existing artwork behavior.
- Adaptive colors remained consistent through 720×480, 1600×900, 1280×800, and back to 720×480 without reloading. Picker colors also remained consistent during resize.
- Compact picker and hover-fade screenshots were inspected.
- Native macOS resize behavior still requires verification after loading the finished package. The app was playing during delivery preparation and was left uninterrupted.

This package includes the previously tested subtle wet-tail stereo spread. Source files, collections, preferences, queues, and the embedded runtime remain preserved. Windows/Linux are unverified.

## Packaged delivery

Executable SHA-256: `907ae93f441d2ed61c00f07726fb3377e4066251f9d6c0032b6b7198c1b3d1e0`. Deep ad-hoc signature and strict verification passed. Embedded sidecar and ONNX runtime hashes match the previous build. Playback was paused at 2:06 before the update was loaded.

After loading, the native app restored 443 songs, existing collections, and the paused 2:06 position. Select song opened the separate themed picker with real library artwork; choosing a song returned to creation with that seed. Closed the unsaved test creation without generating or saving a collection. Native resize acceptance remains pending; automated live-resize checks passed.
