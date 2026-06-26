#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

IMAGE="${MUSICOS_BUILD_IMAGE:-musicos-buildroot:24.04}"
OUT_VOLUME="${MUSICOS_BUILD_OUT_VOLUME:-musicos-buildroot-out}"

if ! docker info >/dev/null 2>&1; then
  if command -v colima >/dev/null 2>&1; then
    colima start
  fi
fi

docker build --platform linux/amd64 -f "${ROOT}/docker/buildroot.Dockerfile" -t "${IMAGE}" "${ROOT}"

exec docker run --rm \
  --platform linux/amd64 \
  -v "${ROOT}:/work" \
  -v "${OUT_VOLUME}:/work/out" \
  -v "${ROOT}/out:/host-out" \
  -w /work \
  -e BUILDROOT_VERSION="${BUILDROOT_VERSION:-2026.05}" \
  "${IMAGE}" \
  bash -lc '
    set -euo pipefail
    export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    if [ -d out/qemu ]; then
      find out/qemu/host out/qemu/target -type d -exec chmod 0755 {} + 2>/dev/null || true
      for dir in \
        out/qemu/host/bin \
        out/qemu/host/sbin \
        out/qemu/host/libexec \
        out/qemu/target/bin \
        out/qemu/target/sbin \
        out/qemu/target/usr/bin \
        out/qemu/target/usr/sbin
      do
        [ -d "${dir}" ] && find "${dir}" -type f -exec chmod 0755 {} +
      done
    fi
    rm -rf out/qemu/build/buildroot-config out/qemu/Makefile
    scripts/build-qemu.sh
    mkdir -p /host-out/qemu
    rm -rf /host-out/qemu/images
    cp -a /work/out/qemu/images /host-out/qemu/images
    cp -f /work/out/qemu/.config /host-out/qemu/.config
  '
