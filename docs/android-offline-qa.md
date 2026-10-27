# Android / Devices delivery evidence

This pass adds the Android offline app and opt-in desktop Wi-Fi sharing alongside the previously committed adaptive desktop shell. It keeps the embedded OS API separate. The personal name, collections, playback protocol, installed models and embeddings are not migrated.

## Automated checks

- Desktop UI: 161 tests across 49 files; production UI build passes.
- Native desktop: 166 tests pass. The additional emulator fixture is ignored by ordinary test runs and runs only with an explicit fixture-only directory.
- Chromium: 13 interaction tests pass, including the 48 layout combinations at 480×480, 720×480 and 1280×800, all interface scales, independent navigation/Search history, retained Settings drafts, fixed More-button geometry, queue identities, nested Escape, focus restoration, delayed artwork and Reduced Motion.
- WebKit: Wi-Fi opt-in / bounded layout and finite playful reactions pass. The earlier adaptive-shell WebKit matrix and modal/keyboard tests are recorded in adaptive-shell-qa.md.
- Android JVM: five tests pass for private pairing links, manifest identity/size validation, ordered duplicates, transactional imports and retained phone favorites, pinned TLS, byte-range resume, checksum failure and wrong-certificate rejection. Android lint and signed release build pass.
- Android 17/API 37 emulator: background playback, one persistent session, duplicate queue identities, moving/removing entries, Activity recreation and paused restoration pass. Phone favorite changes reach the active system media metadata without resetting playback or queue identities; artwork clears stale images before decoding a changed source. Light/Dark navigation and retained Search pass. PNGs from the instrumented UI were inspected; theme fills, Figtree, Sketch Library/Home, artwork bounds and transport remain coherent.
- Cross-platform transfer: the real Rust TLS fixture and Android SyncWorker pair/approve, copy PCM audio and JPEG artwork, verify hashes, import a duplicate collection sequence, acknowledge the revision, stop the synthetic Mac server, and play the verified offline song on the phone. The media transport notification is checked in that test. It never opens the user's library or profile.

Logs and test reports are under the worktree's ignored out/android-qa directory and Android build reports. Instrumented screenshots are under out/persistent-qa/android-final. No pairing links, credentials, signing keys or private profile checkpoints belong in a public release archive.

## Playful presentation

Home uses the supplied face icon and a warmer prompt. Explicit navigation and creation actions give their icon a 180 ms tilt and settle, with stationary labels/hit areas/artwork. Android buttons soften inward by 2% over 140 ms; navigation uses the same small reaction. There are no permanent decorative frame loops. Desktop reactions reverse from their displayed transform, cancel on resize/scroll/hidden pages/unmount and disappear after completion. Reduced Motion skips the icon reactions. More and menu presentation keep their existing single transform owner.

The prior installed-vs-adaptive browser comparison remains approximately 16.7 ms median / 16.8 ms p95 in that harness; layout 25.4→23.9 ms, callback work 7.2→6.7 ms, scripting 369→387 ms (single-run variability). No active animations remained after settling. These are browser-fixture timings, not native display pacing or a 120 fps claim.

## Acceptance boundaries

The emulator runs without audio output to protect current Mac listening. Physical phone touch, Bluetooth/headset routing, actual Android Auto hardware/certification, OEM battery restrictions and system equalizers remain real-device gates. Windows/Linux are separate desktop gates. The APK is signed with a dedicated update key retained privately outside the repository; distribution is local, not a Play Store publication.

Final app / APK hashes, unchanged protected runtime/model hashes and the private latest-profile checkpoint are recorded in the delivery's release.json. Installation requires a fresh paused/stopped check and preserves the previous app. Until that record says installed/post_load_verified, source/build completion must not be reported as a completed installation.

## Installed native check — October 27, 2026

Installed after a fresh native paused check and a private latest-profile checkpoint. MusicOS restored Kool Nuit at 122.2 seconds, paused. Foreground Home and album pages render with cached artwork; Devices starts with sharing off. Its selection menu contains the complete 13 saved collections and restores focus after Escape. The library retains 508 songs and 35 favorites; manual playlist rows/sequences and all saved preferences match the checkpoint. The original signed app is retained. The background window initially displayed its suspended startup render; bringing it forward resolved this without a production code change.

Signed Mac executable SHA-256: `493901d4871235d159c2da757f7b4d051f52e1e63c4bc1157043e82a23440239`.
Signed Android APK SHA-256: `e96a749caf2b172fc427106075f47ecf58c34059859c546b7a5b93b4bc38ab6f`.
All eight protected embedded runtime/model files match the previous installation. The OS checkout remains unchanged. Release files and the complete hash record are in `releases/desktop-android-2026-10-27` in the primary workspace. Physical Android/route/Auto and Windows/Linux gates remain separate.
