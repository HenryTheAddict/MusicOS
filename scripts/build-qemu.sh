#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/env.sh"
BUILDROOT="${BUILDROOT_DIR:-${ROOT}/vendor/buildroot}"

if [[ ! -d "${BUILDROOT}" ]]; then
  BUILDROOT="$("${ROOT}/scripts/fetch-buildroot.sh")"
fi

make -C "${BUILDROOT}" \
  O="${ROOT}/out/qemu" \
  BR2_EXTERNAL="${ROOT}/buildroot-external" \
  musicos_qemu_defconfig

make -C "${BUILDROOT}" O="${ROOT}/out/qemu"
