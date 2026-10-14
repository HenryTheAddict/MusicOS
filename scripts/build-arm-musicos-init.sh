#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rootfs="$repo_root/out/rg35xx/knulli-ui-root"
pkgconfig="$repo_root/out/rg35xx/pkgconfig"
zig_cache="$repo_root/out/rg35xx/zig-cache"

if [[ ! -d "$rootfs/usr/lib" ]]; then
  echo "missing KNULLI rootfs libs at $rootfs/usr/lib" >&2
  exit 1
fi

if [[ ! -f "$rootfs/usr/lib/libasound.so" ]]; then
  echo "missing ALSA runtime lib at $rootfs/usr/lib/libasound.so" >&2
  exit 1
fi

mkdir -p "$pkgconfig"
mkdir -p "$zig_cache/global" "$zig_cache/local"
cat >"$pkgconfig/alsa.pc" <<PC
prefix=/usr
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: alsa
Description: ALSA runtime from the RG35XX KNULLI rootfs
Version: 1.2.14
Libs: -L\${libdir} -lasound
Cflags:
PC

# Homebrew's Rust toolchain is missing the aarch64-linux std libraries and shadows
# rustup on PATH. Prefer the rustup-managed toolchain so the cross target resolves.
if command -v rustup >/dev/null 2>&1; then
  if rustc_path="$(rustup which rustc 2>/dev/null)"; then
    export PATH="$(dirname "$rustc_path"):$PATH"
  fi
fi

export CC_aarch64_unknown_linux_gnu="$repo_root/scripts/zig-aarch64-linux-gnu-cc"
export AR_aarch64_unknown_linux_gnu="zig ar"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$repo_root/scripts/zig-aarch64-linux-gnu-cc"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export ZIG_GLOBAL_CACHE_DIR="$zig_cache/global"
export ZIG_LOCAL_CACHE_DIR="$zig_cache/local"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$rootfs"
export PKG_CONFIG_PATH="$pkgconfig"
export RUSTFLAGS="-L native=$rootfs/lib -L native=$rootfs/usr/lib -C link-arg=-Wl,--allow-shlib-undefined"

cargo build \
  -p musicos-init \
  --bin musicos-init \
  --target aarch64-unknown-linux-gnu \
  --release \
  --features pid1,device-audio

file "$repo_root/target/aarch64-unknown-linux-gnu/release/musicos-init"
