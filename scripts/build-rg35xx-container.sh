#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

IMAGE="${MUSICOS_BUILD_IMAGE:-musicos-buildroot:24.04}"
OUT_VOLUME="${MUSICOS_BUILD_OUT_VOLUME:-musicos-buildroot-out}"
if command -v sysctl >/dev/null 2>&1; then
  HOST_CPUS="$(sysctl -n hw.logicalcpu 2>/dev/null || true)"
fi
if [[ ! "${HOST_CPUS:-}" =~ ^[0-9]+$ ]]; then
  HOST_CPUS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || printf '4')"
fi
if (( HOST_CPUS > 2 )); then
  DEFAULT_BUILD_JOBS=$((HOST_CPUS - 1))
else
  DEFAULT_BUILD_JOBS="$HOST_CPUS"
fi
BUILD_JOBS="${MUSICOS_BUILD_JOBS:-$DEFAULT_BUILD_JOBS}"
CONTAINER_CPUS="${MUSICOS_CONTAINER_CPU:-$BUILD_JOBS}"
case "$(uname -m)" in
  arm64|aarch64) DEFAULT_PLATFORM="linux/arm64" ;;
  *) DEFAULT_PLATFORM="linux/amd64" ;;
esac
BUILD_PLATFORM="${MUSICOS_BUILD_PLATFORM:-$DEFAULT_PLATFORM}"
CONTAINER_DNS="${MUSICOS_CONTAINER_DNS:-1.1.1.1}"

BUILD_SCRIPT='
    set -euo pipefail
    export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    buildroot_version="${BUILDROOT_VERSION:-2026.05}"
    export BUILDROOT_DIR="/work/out/buildroot-${buildroot_version}"
    if [ ! -d "$BUILDROOT_DIR" ]; then
      archive="/work/out/buildroot-${buildroot_version}.tar.xz"
      curl -fL "https://buildroot.org/downloads/buildroot-${buildroot_version}.tar.xz" -o "$archive"
      tar -xf "$archive" -C /work/out
    fi
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

RUNTIME="${MUSICOS_CONTAINER_RUNTIME:-auto}"
if [[ "$RUNTIME" == "auto" ]]; then
  if command -v container >/dev/null 2>&1 && container system status >/dev/null 2>&1; then
    RUNTIME="apple"
  else
    RUNTIME="docker"
  fi
fi

if [[ "$RUNTIME" != "apple" && "$RUNTIME" != "docker" ]]; then
  echo "Unsupported container runtime: $RUNTIME (expected apple, docker, or auto)." >&2
  exit 1
fi

if [[ "$RUNTIME" == "apple" ]]; then
  if ! command -v container >/dev/null 2>&1; then
    echo "Apple Container CLI is not installed." >&2
    exit 1
  fi
  container system start --enable-kernel-install >/dev/null
  container builder start \
    --cpus "$CONTAINER_CPUS" \
    --memory "${MUSICOS_CONTAINER_MEMORY:-6G}" \
    --dns "$CONTAINER_DNS" >/dev/null
  container volume create "$OUT_VOLUME" >/dev/null 2>&1 || true
  if [[ "${MUSICOS_REBUILD_IMAGE:-0}" == "1" ]] || ! container image inspect "$IMAGE" >/dev/null 2>&1; then
    container build \
      --platform "$BUILD_PLATFORM" \
      --dns "$CONTAINER_DNS" \
      --file "${ROOT}/docker/buildroot.Dockerfile" \
      --tag "$IMAGE" \
      "$ROOT"
  fi
  exec container run --rm \
    --platform "$BUILD_PLATFORM" \
    --dns "$CONTAINER_DNS" \
    --cpus "$CONTAINER_CPUS" \
    --memory "${MUSICOS_CONTAINER_MEMORY:-6G}" \
    -v "${ROOT}:/work" \
    -v "${OUT_VOLUME}:/work/out" \
    -v "${ROOT}/out:/host-out" \
    -w /work \
    -e BUILDROOT_VERSION="${BUILDROOT_VERSION:-2026.05}" \
    -e MUSICOS_BUILD_JOBS="$BUILD_JOBS" \
    "$IMAGE" \
    bash -lc "$BUILD_SCRIPT"
fi

if ! docker info >/dev/null 2>&1; then
  if command -v colima >/dev/null 2>&1; then
    colima start --cpu "${MUSICOS_COLIMA_CPU:-4}" --memory "${MUSICOS_COLIMA_MEMORY:-6}"
  else
    echo "Docker is not running and Colima is not installed." >&2
    exit 1
  fi
fi

if [[ "${MUSICOS_REBUILD_IMAGE:-0}" == "1" ]] || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  docker build --platform "$BUILD_PLATFORM" -f "${ROOT}/docker/buildroot.Dockerfile" -t "$IMAGE" "$ROOT"
fi
exec docker run --rm \
  --platform "$BUILD_PLATFORM" \
  -v "${ROOT}:/work" \
  -v "${OUT_VOLUME}:/work/out" \
  -v "${ROOT}/out:/host-out" \
  -w /work \
  -e BUILDROOT_VERSION="${BUILDROOT_VERSION:-2026.05}" \
  -e MUSICOS_BUILD_JOBS="$BUILD_JOBS" \
  "$IMAGE" \
  bash -lc "$BUILD_SCRIPT"
