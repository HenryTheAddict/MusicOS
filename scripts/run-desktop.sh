#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

export MUSICOS_COMPANION_PORT="${MUSICOS_COMPANION_PORT:-18080}"
export MUSICOS_MEDIA_DB="${MUSICOS_MEDIA_DB:-out/desktop-media.sqlite3}"
export MUSICOS_SYNC_TARGET="${MUSICOS_SYNC_TARGET:-http://127.0.0.1:${MUSICOS_COMPANION_PORT}}"
export MUSICOS_SYNC_DIR="${MUSICOS_SYNC_DIR:-./music}"

mkdir -p out "$MUSICOS_SYNC_DIR"

if [ ! -d node_modules/electron ]; then
  npm install
fi

echo "Starting MusicOS Control Center at ${MUSICOS_SYNC_TARGET}..."
npm run sync:electron
