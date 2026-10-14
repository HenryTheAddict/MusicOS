#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

export MUSICOS_QEMU_NET="${MUSICOS_QEMU_NET:-sync}"
export MUSICOS_QEMU_HOST_PORT="${MUSICOS_QEMU_HOST_PORT:-18080}"
export MUSICOS_SYNC_TARGET="${MUSICOS_SYNC_TARGET:-http://127.0.0.1:${MUSICOS_QEMU_HOST_PORT}}"
export MUSICOS_SYNC_DIR="${MUSICOS_SYNC_DIR:-./music}"

mkdir -p "${MUSICOS_SYNC_DIR}"

scripts/run-qemu.sh &
qemu_pid=$!

cleanup() {
  kill "${qemu_pid}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "Waiting for QEMU MusicOS API at ${MUSICOS_SYNC_TARGET}..."
for _ in $(seq 1 80); do
  if curl -fsS "${MUSICOS_SYNC_TARGET}/api/status" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

MUSICOS_SYNC_TARGET="${MUSICOS_SYNC_TARGET}" \
MUSICOS_SYNC_DIR="${MUSICOS_SYNC_DIR}" \
cargo run -p musicos-desktop --bin musicos-control-center
