#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/env.sh"
BUILDROOT="${BUILDROOT_DIR:-${ROOT}/vendor/buildroot}"

if [[ ! -d "${BUILDROOT}" ]]; then
  BUILDROOT="$("${ROOT}/scripts/fetch-buildroot.sh")"
fi

make -C "${BUILDROOT}" \
  O="${ROOT}/out/rg35xx" \
  BR2_EXTERNAL="${ROOT}/buildroot-external" \
  musicos_rg35xx_defconfig

BUILD_DIR="${ROOT}/out/rg35xx"
BUILD_LOG="${BUILD_DIR}/build.log"
BUILD_JOBS="${MUSICOS_BUILD_JOBS:-1}"

build_progress() {
  local build_pid="$1"
  local percent phase objects

  while kill -0 "$build_pid" 2>/dev/null; do
    percent=5
    phase="configuring"

    if [[ -x "${BUILD_DIR}/host/bin/aarch64-buildroot-linux-musl-gcc" ]]; then
      percent=12
      phase="toolchain ready"
    fi

    if compgen -G "${BUILD_DIR}/build/linux-*" >/dev/null; then
      objects="$(find "${BUILD_DIR}"/build/linux-* -type f -name '*.o' 2>/dev/null | wc -l | tr -d ' ')"
      percent=$((15 + objects / 45))
      (( percent > 82 )) && percent=82
      phase="compiling kernel (${objects} objects)"
    fi

    if compgen -G "${BUILD_DIR}/build/linux-*/.stamp_installed" >/dev/null; then
      percent=84
      phase="building target packages"
    fi

    if [[ -f "${BUILD_DIR}/images/Image" && "${BUILD_DIR}/images/Image" -nt "$BUILD_LOG" ]]; then
      percent=86
      phase="assembling target filesystem"
    fi
    if [[ -f "${BUILD_DIR}/build/musicos-init/.stamp_installed" ]]; then
      percent=92
      phase="MusicOS init installed"
    fi
    if [[ -f "${BUILD_DIR}/images/rootfs.tar" && "${BUILD_DIR}/images/rootfs.tar" -nt "$BUILD_LOG" ]]; then
      percent=98
      phase="finalizing images"
    fi

    printf '[build] ~%3d%%  %s  (%s parallel jobs)\n' "$percent" "$phase" "$BUILD_JOBS"
    sleep 10
  done
}

printf '[build] starting with %s parallel jobs; detailed log: %s\n' "$BUILD_JOBS" "$BUILD_LOG"
make -C "${BUILDROOT}" O="${BUILD_DIR}" BR2_JLEVEL="$BUILD_JOBS" >"$BUILD_LOG" 2>&1 &
build_pid=$!
build_progress "$build_pid" &
progress_pid=$!

if wait "$build_pid"; then
  kill "$progress_pid" 2>/dev/null || true
  wait "$progress_pid" 2>/dev/null || true
  printf '[build] 100%%  complete\n'
else
  status=$?
  kill "$progress_pid" 2>/dev/null || true
  wait "$progress_pid" 2>/dev/null || true
  printf '[build] failed; last 80 log lines follow\n' >&2
  tail -n 80 "$BUILD_LOG" >&2
  exit "$status"
fi
