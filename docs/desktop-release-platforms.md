# Desktop release platforms

The manually dispatched Desktop release workflow builds on the native architecture:

| Updater key | Runner | Assets |
| --- | --- | --- |
| `darwin-aarch64` | `macos-15` (Apple Silicon) | DMG, `.app.tar.gz`, updater signature |
| `darwin-x86_64` | `macos-15-intel` | DMG, `.app.tar.gz`, updater signature |
| `linux-x86_64` | `ubuntu-22.04` | AppImage, updater signature, Arch `.pkg.tar.zst` |
| `windows-x86_64` | `windows-2025` | NSIS installer, updater signature |

All four updater payloads require a valid Minisign signature with the release
version in its signed trusted comment. The final verification job requires the
exact asset set, checks Arch metadata and launchers, writes `latest.json`, and
generates and verifies `SHA256SUMS` covering every download (including signatures
and `latest.json`). Arch packages are not AppImage updater payloads; install or
upgrade with `sudo pacman -U musicos-<version>-1-x86_64.pkg.tar.zst`.

## Native resources and Mac minimum OS

Both Mac releases declare **macOS 14.0**. This is also the deployment target for
Rust, Objective-C and the Intel inference runtime build. Before this change,
neither `src-tauri/tauri.conf.json` nor `tauri.macos.conf.json` set
`minimumSystemVersion`, and the workflow did not set `MACOSX_DEPLOYMENT_TARGET`.
Tauri's default is 10.13, but that was not a truthful effective minimum:

- The tracked `src-tauri/resources/runtime/libonnxruntime.1.31.0.dylib` is a
  single ARM64 Mach-O with `LC_BUILD_VERSION`, platform macOS, `minos 14.0`.
  The official [1.31.0 PyPI distribution](https://pypi.org/pypi/onnxruntime/1.31.0/json)
  likewise names its Mac wheels `macosx_14_0_arm64`.
- Neither that distribution nor the official
  [Microsoft.ML.OnnxRuntime 1.31.0 NuGet package](https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/1.31.0/microsoft.ml.onnxruntime.1.31.0.nupkg)
  contains an Intel Mac library. The Intel job builds CPU ONNX Runtime from the
  release commit `402979ac1bef89dcd55add41eaa7277c407599eb`, recorded in that
  package's `.nuspec` repository and release metadata. It sets
  `CMAKE_OSX_ARCHITECTURES=x86_64` and `CMAKE_OSX_DEPLOYMENT_TARGET=14.0`, and
  stages the resulting versioned dylib and upstream license notices.
  Intel release builds load the copy in `Contents/Frameworks`, which Tauri signs
  with the application's identity, rather than the legacy Resources copy.
  The workflow verifies both signatures and matching Team IDs (including the
  ad-hoc case). Development and Apple Silicon runtime paths are unchanged.
  See the upstream [native CPU build instructions](https://onnxruntime.ai/docs/build/inferencing.html).
- The workflow checks the staged dylib's native architecture with `lipo` and its
  minimum OS with `otool` before packaging. Source checkout, compilation and the
  resulting Intel load commands still need a real Mac release run for validation.
- There are **no bundled FFmpeg executables or dylibs** in the existing desktop
  resource configuration. Audio decoding uses Rust/rodio/Symphonia. Optional
  Wi-Fi Vorbis conversion discovers an external FFmpeg on PATH or at
  `/opt/homebrew/bin/ffmpeg` / `/usr/local/bin/ffmpeg`; its OS requirements depend
  on the user's installation, not the release bundle. This behavior is unchanged.
- `scripts/prepare-desktop.mjs` builds the sidecar natively and copies it to
  `src-tauri/binaries/musicos-mac-<host triple>`. Thus the Intel runner supplies
  `musicos-mac-x86_64-apple-darwin`, not the ARM64 executable. The same verified
  Sparklist model is architecture-independent.

The Intel runner is documented in GitHub's
[hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
and [Intel macOS 15 announcement](https://github.com/actions/runner-images/issues/13045).
Both Mac jobs use Developer ID signing when configured, otherwise they fall back
to ad-hoc signing with `APPLE_SIGNING_IDENTITY=-`. Notarization is available when
the Apple credentials are configured, but is not required. After bundling,
`codesign --verify --deep --strict` validates the Intel application with either
signature type.

## Arch packaging

The Linux job repackages its already-built AppImage using
`scripts/linux/arch/PKGBUILD` in `archlinux:latest`. `makepkg` runs unprivileged,
verifies the input AppImage checksum, extracts without FUSE and retains the
AppDir, its bundled model/runtime and `AppRun` under `/opt/musicos`. The package
adds a launcher, desktop entry, icons and native Arch dependencies. The container
installs the result with `pacman -U` and checks its installed files with
`pacman -Qkk` before upload. It does not rebuild or modify the signed AppImage.

Local release-tool regression tests: `node --test scripts/desktop-release.test.mjs`
(requires `minisign`, `tar` with zstd support, and `sha256sum`). Real signing,
notarization, GUI launch and updater installation require platform release tests.
