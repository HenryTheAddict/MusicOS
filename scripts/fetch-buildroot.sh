#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/env.sh"
VERSION="${BUILDROOT_VERSION:-2026.05}"
ARCHIVE="buildroot-${VERSION}.tar.xz"
URL="https://buildroot.org/downloads/${ARCHIVE}"

mkdir -p "${ROOT}/vendor"
cd "${ROOT}/vendor"

if [[ ! -d "buildroot-${VERSION}" ]]; then
  if [[ ! -f "${ARCHIVE}" ]]; then
    curl -L -o "${ARCHIVE}" "${URL}"
  fi
  tar -xf "${ARCHIVE}"
fi

ln -sfn "buildroot-${VERSION}" "${ROOT}/vendor/buildroot"
echo "${ROOT}/vendor/buildroot"
