# Intel/AMD Linux AppImage

Build on x86_64 Ubuntu 22.04 (glibc 2.35) or the pinned container recipe:

```
docker build --platform linux/amd64 -f scripts/linux/Dockerfile -t musicos-appimage-builder .
```

Copy this desktop worktree to a writable Linux workspace, excluding `target`, all `node_modules`, `out`, `.git`, `src-tauri/binaries`, and `src-tauri/resources/runtime`. Include the existing `src-tauri/resources/models/clap-general-v1` files. Run `scripts/linux/build-appimage.sh` in the image with that workspace mounted at `/workspace`. On Apple Silicon this requires an amd64 emulator; a native Intel/AMD Linux builder is faster. The script uses a pinned packaging plugin and a builder-only QEMU bridge for static AppImage runtimes that Rosetta cannot execute. The delivered app remains a normal x86_64 AppImage.

The script verifies the existing model against its manifest, stages the official checksum-pinned ONNX Runtime 1.31.0 Linux CPU libraries, builds the existing desktop sidecar, then packages the application. It never replaces Mac runtime libraries or automatically reprocesses music. Runtime/download/build checksums are written under `out/linux-runtime`. Model preparation is deliberately explicit: a missing model is an error, never a silent fallback download.

The result is `target/release/bundle/appimage/musicOS_0.1.0_amd64.AppImage`. Make it executable and open it as a normal user. Recent Ubuntu/Fedora/Debian distributions need working graphics, audio, and a desktop session. If FUSE is unavailable, use `APPIMAGE_EXTRACT_AND_RUN=1 ./musicOS_0.1.0_amd64.AppImage`. Do not disable the WebKit sandbox to launch it.

## Controller

Connect a standard Xbox/PlayStation-style controller by USB or supported Bluetooth. Settings → General → Controller reports the actual mapping and vibration capability. Controls affect only the focused musicOS window.

| Button | Action |
| --- | --- |
| D-pad / left stick | Move focus |
| A / Cross | Select |
| B / Circle | Close a menu/dialog, or Back |
| Start / Options | Play / pause |
| X / Square | Favorite the playing song |
| Y / Triangle | Cycle queue, lyrics, normal |
| LB / RB | Previous / next |
| LT / RT | Seek backward / forward 10 seconds |
| Right stick | Scroll |
| Right stick click | Item context menu |
| View / Share | Visualizer, on supported platforms |

Navigation, toggles, favorites, drag destinations, and completed drops can provide short vibration ticks. Focusing the playing song’s small heart follows the same acknowledged BPM/phase clock as its animation; unknown BPM remains still. Feedback is optional, stops on blur/pause/track change, and is capped at four beats per second. Linux force feedback requires support and access provided by the controller driver. macOS Force Touch feedback remains separate.

## Acceptance boundaries

AppImage packaging and headless launch checks do not establish physical sound output or controller feel. On the target PC, check USB/Bluetooth reconnection, stick dead zones, dialog/menu focus, Space and keyboard use, real audio output/headphones, rumble, and BPM alignment. Native Metal visualizers, Core ML photo depth, Apple embedded sign-in, and macOS media/Force Touch integrations are not Linux acceptance claims; platform-specific features retain their unavailable/fallback behavior. Static artwork, normal library/playback/lyrics, and Wi-Fi workflows do not depend on those integrations.
