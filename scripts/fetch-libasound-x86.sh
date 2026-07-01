#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
URL="http://ftp.us.debian.org/debian/pool/main/a/alsa-lib/libasound2_1.2.8-1+b1_amd64.deb"
ARCHIVE="libasound2_amd64.deb"

mkdir -p "${ROOT}/vendor/x86_64-linux-gnu"
cd "${ROOT}/vendor/x86_64-linux-gnu"

if [[ ! -f "libasound.so.2" ]]; then
  echo "Downloading libasound deb package..."
  curl -L -o "${ARCHIVE}" "${URL}"

  echo "Extracting deb package..."
  ar x "${ARCHIVE}"
  tar -xf data.tar.xz

  # Find and copy libasound.so
  cp usr/lib/x86_64-linux-gnu/libasound.so.2.0.0 ./libasound.so
  ln -sf libasound.so libasound.so.2

  # Clean up temp files
  rm -rf usr debian-binary control.tar.xz data.tar.xz "${ARCHIVE}"
fi

echo "x86_64 libasound.so ready at ${ROOT}/vendor/x86_64-linux-gnu"
