# Linux AppImage and controller verification — 2026-11-04

Code: `460d8b8fa31a02b84bc081db017bcb7e367f38c1`, isolated `codex/desktop-heart-preservation` branch.

Artifact: `musicOS_0.1.0_amd64.AppImage` (Intel/AMD x86_64).

SHA-256: `d23c8cd57b2c348bbe68bf68879c0bad91be36b00a6fcd25920784144c296e79`.

## Implemented

- Ubuntu 22.04 / glibc 2.35 AppImage packaging, with Linux ONNX Runtime 1.31.0 and the existing checksum-verified CLAP model. Mac runtime resources remain platform-specific.
- Fixed four Linux desktop audio compilation guards without enabling the OS/device ALSA path in the desktop build.
- Native Gilrs input: directional focus, selection, Back, playback, previous/next, seek, favorite, queue/lyrics cycling, scrolling, and focused-item actions. Popup activation uses its keyboard handler, avoiding synthetic-click-only failures.
- Optional native force feedback for interactions and the acknowledged BPM/phase clock. One bounded scheduler; no audio-callback work, queued beats, or feedback after focus loss. Unknown BPM stays still. Hardware mapping/vibration capabilities appear in Settings → General → Controller.
- Controller focus respects menus, dialogs, disclosures, sliders, and Settings tabs. Controller interaction reveals player chrome; focused controls hold it open.

## Passed

- 242 UI unit checks across 75 files.
- Five controller browser checks: 480×480, 720×480, 1280×800; menu activation/dismissal and focus restoration; real slider input/change delivery; Settings tab/disclosure navigation.
- Native mapping/feedback gate/preference persistence tests (3), and platform runtime-path test (1), compiled on macOS. Linux production compilation passed.
- Exact final AppImage launched as an ordinary Linux user through its extraction/run entry point in the emulated Linux desktop. WebKit sandbox disabling was not used. Software rendering was selected for the virtual display; this is not a native GPU performance result.
- Actual Linux interface opened the song library and Controller settings, and resized to 480, 720, and 1280 px. Screenshots are included in the release folder.
- Synthetic 220 Hz audio played through the real packaged playback engine into a paced PulseAudio null sink and monitor. Capture peak 0.1831; 212,744 nonzero samples; first five seconds silent at startup; tail silent after acknowledged Pause. This establishes digital output, not a physical listening or headphone test.
- No unresolved linked libraries in the main executable, desktop sidecar, or inference runtime. Packaged Linux ONNX Runtime loaded and reported version 1.31.0.
- All 39 changed source files matched the recorded build input. Source, dependency, runtime, artifact, and preservation manifests accompany the release.
- Existing Mac signed executable and all eight protected embedded runtime/model files unchanged. Primary OS worktree diff unchanged. No Mac installation, playback command, profile migration, or collection rewrite was performed.

## Hardware acceptance still required

Connect the standard controller to the Intel/AMD PC and verify USB/Bluetooth reconnect, focus/repeat behavior, real speaker/headphone output, rumble availability, and beat alignment/feel. No physical Linux controller was available in this environment. Vibration is driver-dependent; unsupported controllers retain navigation without rumble.

Native Metal visualizers, Core ML depth, Apple embedded authentication, and macOS-specific media/Force Touch integration are not Linux support claims. Their existing unavailable/static-fallback behavior remains. No native Linux GPU or performance benchmark is claimed.

## Rebuilding

See `scripts/linux/README.md` and `scripts/linux/build-appimage.sh`. The builder pins its packaging helper and Linux runtime archives. The static-runtime bridge is build-only for Rosetta; it is not included in the delivered app. The temporary build container and VM are stopped after delivery; reusable build caches are retained.
