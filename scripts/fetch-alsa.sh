#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="1.2.12"
ARCHIVE="alsa-lib-${VERSION}.tar.gz"
URL="https://github.com/alsa-project/alsa-lib/archive/refs/tags/v${VERSION}.tar.gz"

mkdir -p "${ROOT}/vendor"
cd "${ROOT}/vendor"

if [[ ! -d "alsa-lib-${VERSION}" ]]; then
  if [[ ! -f "${ARCHIVE}" ]]; then
    echo "Downloading ALSA headers..."
    curl -L -o "${ARCHIVE}" "${URL}"
  fi
  echo "Extracting ALSA headers..."
  tar -xf "${ARCHIVE}"
fi

ln -sfn "alsa-lib-${VERSION}" "${ROOT}/vendor/alsa-lib"
echo "ALSA headers fetched to ${ROOT}/vendor/alsa-lib"
