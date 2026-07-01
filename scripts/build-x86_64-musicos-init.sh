#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pkgconfig="$repo_root/out/x86_64/pkgconfig"
zig_cache="$repo_root/out/x86_64/zig-cache"

mkdir -p "$pkgconfig"
mkdir -p "$zig_cache/global" "$zig_cache/local"

cat >"$pkgconfig/alsa.pc" <<PC
Name: alsa
Description: ALSA runtime for x86_64 cross-compile
Version: 1.2.12
Libs: -L${repo_root}/vendor/x86_64-linux-gnu -lasound
Cflags: -I${repo_root}/vendor/alsa-lib/include
PC

export CC_x86_64_unknown_linux_gnu="$repo_root/scripts/zig-x86_64-linux-gnu-cc"
export AR_x86_64_unknown_linux_gnu="zig ar"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$repo_root/scripts/zig-x86_64-linux-gnu-cc"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export ZIG_GLOBAL_CACHE_DIR="$zig_cache/global"
export ZIG_LOCAL_CACHE_DIR="$zig_cache/local"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH="$pkgconfig"
export RUSTFLAGS="-L native=$repo_root/vendor/x86_64-linux-gnu -C link-arg=-Wl,--allow-shlib-undefined"

cargo build \
  -p musicos-init \
  --bin musicos-init \
  --target x86_64-unknown-linux-gnu \
  --release \
  --features audio

file "$repo_root/target/x86_64-unknown-linux-gnu/release/musicos-init"
