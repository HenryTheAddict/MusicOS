# Embedded QEMU boot and playback baseline

| Path | What runs |
| --- | --- |
| QEMU Buildroot | The `musicos_qemu_defconfig` aarch64 kernel on QEMU `virt`; Rust `/sbin/init` as PID 1, a writable ext4 image, virtio GPU/evdev/block/sound, and a forwarded companion API. |
| Anbernic RG35XX 2024 / Allwinner H700 | The current KNULLI squashfs payload runs the device build against the hardware's kernel and ALSA devices. `musicos_rg35xx_defconfig` is a separate mainline bring-up target, not the QEMU image. |
| Host previews | The native `musicos-mac`/preview sidecar and desktop Control Center run as ordinary host processes. Their rendering and API checks do not cover boot, guest mounts, or physical input/audio. |

## Reproduce on a Linux host

Install the Buildroot dependencies in [the build container](../docker/buildroot.Dockerfile)
or use `scripts/build-qemu-container.sh`. The smoke test needs Python 3 and an
aarch64 QEMU build with `virtio-sound-pci`, user networking, QMP PNG screen dumps,
and the WAV audio backend. Check sound-device availability with
`qemu-system-aarch64 -device virtio-sound-pci,help`; older distributions may not
ship it. No KVM, X server, or host audio device is needed.

```sh
set -o pipefail
mkdir -p out
env -u BASH_ENV scripts/build-qemu.sh 2>&1 | tee out/qemu-build.log
python3 scripts/verify-qemu.py out/qemu-smoke-640
python3 scripts/verify-qemu.py --resolution 1280x960 out/qemu-smoke-1280
```

On Devin Linux, keeping its inherited `BASH_ENV` can make Buildroot fail at
`host-skeleton` before compiling the guest. Clearing it is a host workaround.
Use `--qemu /path/to/qemu-system-aarch64` for an alternate QEMU and `--port`
for a different loopback port. Use a fresh Buildroot image and a new results
directory for each run.

The smoke test copies the image, forwards the companion API on loopback only,
completes setup with QMP-injected virtio keyboard events, uploads one generated
WAV and waits for SQLite to return it. It starts the track from the guest's Songs
screen, checks the ALSA playback position and non-silent samples captured by
QEMU, and changes the volume through the guest Settings screen.

The test then sends QEMU `quit` and boots the same ext4 image again, without
asking the guest to flush or shut down. It checks the saved setup flag, volume,
track ID, file size, play count, and last-played timestamp. Resending the same
upload must return `skipped`, which checks the file contents by SHA-256, not just
what SQLite reports. The play count distinguishes persisted metadata from a
freshly reindexed file. This simulates an abrupt VM stop, not the behavior of a
specific handheld power controller or SD card.

On success the result directory contains the exact QEMU command lines, both
serial logs, actual QEMU framebuffer screenshots, companion responses, and the
QEMU WAV capture plus its PCM summary. On failure, retain the partial directory
and terminal error; compare the QEMU stderr log with the guest serial log before
classifying it as a launch, boot, or runtime failure.

## Reproduce the storage failure

Build the QEMU image from the parent revision of the durability change in a
separate checkout. Run the **new** verifier against the old image:

```sh
python3 scripts/verify-qemu.py --images ../MusicOS-before/out/qemu/images out/qemu-before
```

The upload and playback succeeded on the original image. Before the host stopped
QEMU, the database reported a 1,920,044-byte track and a play count of one. After
the immediate restart, ext4 journal recovery left the acknowledged upload at
zero bytes, and the database no longer had the recorded play count. Settings
survived. A diagnostic run using `rootflags=sync` preserved both, isolating
pending guest writes rather than a decoder or navigation fault.

The embedded Linux build now flushes upload bytes before rename and directory
metadata before replying, and uses SQLite WAL `synchronous=FULL` on every
library database connection. The PID 1 application does not implement an
orderly power-off or reboot path that can be relied on to flush these writes.
The verifier still stops QEMU immediately; it does
not add a sleep, a guest flush, or a synchronous root mount. Host preview builds
retain their existing SQLite and upload behavior.

## Other QEMU findings

- The stock launcher hard-codes Cocoa and CoreAudio. On the Linux QEMU used
  here, `scripts/run-qemu.sh` (including with `MUSICOS_QEMU_HEADLESS=1`) exits
  on CoreAudio, and `MUSICOS_QEMU_AUDIO=0 scripts/run-qemu.sh` exits on Cocoa.
  The headless verifier does not invoke that launcher.
- Framebuffer drawing and navigation work, but some bundled SVGs fall back
  to placeholders. See the `Failed to render SVG` warnings and frame captures.

## Physical hardware checks pending

QEMU does not verify H700 boot or SD partition mounting, panel scanout,
RG35XX buttons, brightness/battery/suspend behavior, speaker or headphone
routing, USB import/update, or playback latency and durability on real media.
In particular, repeat the acknowledged-upload and history checks on actual
storage with abrupt power loss before claiming handheld durability.
