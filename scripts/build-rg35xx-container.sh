#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

IMAGE="${MUSICOS_BUILD_IMAGE:-musicos-buildroot:24.04}"
OUT_VOLUME="${MUSICOS_BUILD_OUT_VOLUME:-musicos-buildroot-out}"
BUILD_JOBS="${MUSICOS_BUILD_JOBS:-1}"

if ! docker info >/dev/null 2>&1; then
  if command -v colima >/dev/null 2>&1; then
    colima start --cpu "${MUSICOS_COLIMA_CPU:-4}" --memory "${MUSICOS_COLIMA_MEMORY:-6}"
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
  -e MUSICOS_BUILD_JOBS="${BUILD_JOBS}" \
  "${IMAGE}" \
  bash -lc '
    set -euo pipefail
    export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    if [ -d out/rg35xx ] && [ ! -x out/rg35xx/host/bin/aarch64-buildroot-linux-musl-gcc ]; then
      echo "Buildroot output is missing the aarch64 cross compiler; cleaning stale rg35xx output."
      rm -rf out/rg35xx
    fi
    if [ -d out/rg35xx ]; then
      find out/rg35xx/host out/rg35xx/target -type d -exec chmod 0755 {} + 2>/dev/null || true
      for dir in \
        out/rg35xx/host/bin \
        out/rg35xx/host/sbin \
        out/rg35xx/host/libexec \
        out/rg35xx/target/bin \
        out/rg35xx/target/sbin \
        out/rg35xx/target/usr/bin \
        out/rg35xx/target/usr/sbin
      do
        [ -d "${dir}" ] && find "${dir}" -type f -exec chmod 0755 {} +
      done
    fi
    rm -rf out/rg35xx/build/buildroot-config out/rg35xx/Makefile
    scripts/build-rg35xx.sh
    mkdir -p /host-out/rg35xx
    rm -rf /host-out/rg35xx/images
    cp -a /work/out/rg35xx/images /host-out/rg35xx/images
    cp -f /work/out/rg35xx/.config /host-out/rg35xx/.config
  '
