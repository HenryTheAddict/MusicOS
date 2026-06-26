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

make -C "${BUILDROOT}" O="${ROOT}/out/rg35xx" BR2_JLEVEL="${MUSICOS_BUILD_JOBS:-1}"
