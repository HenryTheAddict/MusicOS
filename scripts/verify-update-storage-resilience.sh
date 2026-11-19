#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
WORK="$(mktemp -d)"
MOUNTS=()
cleanup() {
  for ((i=${#MOUNTS[@]}-1; i>=0; i--)); do
    sudo -n umount "${MOUNTS[i]}" 2>/dev/null || true
  done
  rm -rf "$WORK"
}
trap cleanup EXIT
run() {
  local label="$1"; shift
  if "$@"; then printf 'PASS: %s\n' "$label"; else printf 'FAIL: %s\n' "$label"; exit 1; fi
}
cargo test -p musicos-init --test update_storage_resilience --no-run --message-format=json >"$WORK/build.json"
BIN="$(python3 - "$WORK/build.json" <<'PY'
import json, sys
for line in open(sys.argv[1]):
    artifact = json.loads(line)
    if artifact.get('target', {}).get('name') == 'update_storage_resilience' and artifact.get('executable'):
        print(artifact['executable'])
PY
)"
run 'directory validation, interrupted copies, removed root and DB fallback' "$BIN" --nocapture --test-threads=1 --skip external_image_scenario
if ! sudo -n true; then
  echo 'No passwordless sudo: directory-based scenarios only; loop/unplug/full/remount scenarios SKIPPED.'
  exit 0
fi
for tool in mkfs.ext4 mkfs.vfat mount umount; do command -v "$tool" >/dev/null; done
echo 'Passwordless sudo available: exercising disposable ext4 and vfat loop images.'
mkdir -p "$WORK/usb" "$WORK/stage" "$WORK/storage" "$WORK/target"
image() {
  local name="$1" size="$2" type="$3" mountpoint="$4"
  truncate -s "$size" "$WORK/$name.img"
  "mkfs.$type" "$WORK/$name.img" >/dev/null
  local options=loop
  if [[ "$type" == vfat ]]; then options="loop,uid=$(id -u),gid=$(id -g)"; fi
  sudo -n mount -o "$options" "$WORK/$name.img" "$mountpoint"
  MOUNTS+=("$mountpoint")
  if [[ "$type" == ext4 ]]; then sudo -n chown "$(id -u):$(id -g)" "$mountpoint"; fi
}
scenario() {
  MUSICOS_RESILIENCE_ROOT="$WORK" MUSICOS_RESILIENCE_SCENARIO="$1" "$BIN" --exact external_image_scenario --nocapture
}
image usb 192M vfat "$WORK/usb"
image storage 8M ext4 "$WORK/storage"
echo 'Lazy-unmount during update: open file descriptors may finish copying after detach (not a physical I/O fault).'
scenario copy >"$WORK/copy.log" 2>&1 &
PID=$!
detached=0
while kill -0 "$PID" 2>/dev/null; do
  if [[ -f "$WORK/stage/musicos-init.tmp" ]]; then
    sudo -n umount -l "$WORK/usb"
    detached=1
    break
  fi
done
if wait "$PID"; then cat "$WORK/copy.log"; else cat "$WORK/copy.log"; echo 'FAIL: lazy-unmount update'; exit 1; fi
if [[ "$detached" != 1 ]]; then echo 'FAIL: copy finished before lazy-unmount injection'; exit 1; fi
echo 'PASS: lazy-unmount during copy; complete validated update installed'
cat "$WORK/stage/last-update-result"
sudo -n mount -o "loop,uid=$(id -u),gid=$(id -g)" "$WORK/usb.img" "$WORK/usb"
image stage 8M ext4 "$WORK/stage"
run 'full staging filesystem preserves original target' scenario full
cat "$WORK/stage/last-update-result"
run 'full SQLite filesystem returns write error' scenario db-full
sudo -n mount -o remount,ro "$WORK/storage"
run 'read-only DB filesystem starts and surfaces write error' scenario readonly
echo 'PASS: all disposable-image scenarios (host only; physical unplug/power cut still pending)'
