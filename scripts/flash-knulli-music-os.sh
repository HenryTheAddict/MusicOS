#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /dev/diskN" >&2
  echo "example: $0 /dev/disk2" >&2
  exit 2
fi

disk="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image="$repo_root/third_party/knulli/knulli-h700-rg35xx-plus-scarab-20260511.img"
default_payload="$repo_root/out/rg35xx/images/musicos-knulli-ui.squashfs"
if [[ ! -f "$default_payload" ]]; then
  default_payload="$repo_root/out/rg35xx/images/musicos-knulli-root.squashfs"
fi
payload="${MUSICOS_PAYLOAD:-$default_payload}"
bootlogo="${MUSICOS_BOOTLOGO:-$repo_root/out/rg35xx/images/musicos-bootlogo.bmp}"
sample_dir="${MUSICOS_SAMPLE_DIR:-$repo_root/samples/tanger}"
sample_limit="${MUSICOS_SAMPLE_LIMIT:-0}"
audio_format="${MUSICOS_AUDIO_FORMAT:-ogg}"
ogg_quality="${MUSICOS_OGG_QUALITY:-5}"

if [[ ! -f "$image" ]]; then
  echo "missing KNULLI donor image: $image" >&2
  exit 1
fi

if [[ ! -f "$payload" ]]; then
  echo "missing MusicOS squashfs payload: $payload" >&2
  exit 1
fi

if [[ ! "$disk" =~ ^/dev/disk[0-9]+$ ]]; then
  echo "pass the whole disk like /dev/disk2, not a partition." >&2
  exit 1
fi

if ! diskutil info "$disk" | grep -q "Protocol:.*USB\\|Protocol:.*Secure Digital"; then
  echo "$disk does not look like a removable SD/USB device; refusing." >&2
  diskutil info "$disk" >&2 || true
  exit 1
fi

size_bytes="$(diskutil info "$disk" | awk -F'[()]' '/Disk Size|Total Size/ {gsub(/ Bytes/, "", $2); gsub(/[^0-9]/, "", $2); if ($2 != "") { print $2; exit }}')"
if [[ -n "$size_bytes" && "$size_bytes" -gt $((128 * 1024 * 1024 * 1024)) ]]; then
  echo "$disk is larger than 128GB; refusing because that does not look like the RG35XX SD card." >&2
  diskutil info "$disk" >&2 || true
  exit 1
fi

raw_disk="/dev/r$(basename "$disk")"
echo "About to erase and rewrite $disk using the RG35XX/KNULLI boot/display image."
diskutil info "$disk" | awk -F': *' '/Device Node|Device Location|Protocol|Disk Size|Total Size|Volume Name|Media Name/ {print "  " $1 ": " $2}'
echo "MusicOS payload: $payload"
if [[ -d "$sample_dir" ]]; then
  if [[ "$sample_limit" == "0" ]]; then
    echo "MusicOS samples: $sample_dir (all supported files)"
  else
    echo "MusicOS samples: $sample_dir (limit $sample_limit)"
  fi
fi

osascript - "$disk" "$raw_disk" "$image" <<'APPLESCRIPT'
on run argv
  set diskName to item 1 of argv
  set rawDiskName to item 2 of argv
  set imagePath to item 3 of argv
  set shellCommand to "/usr/sbin/diskutil unmountDisk " & quoted form of diskName & "; /bin/dd if=" & quoted form of imagePath & " of=" & quoted form of rawDiskName & " bs=4m conv=sync; /bin/sync"
  do shell script shellCommand with administrator privileges with prompt "MusicOS needs admin permission to write the RG35XX/KNULLI boot/display image to " & diskName & "."
end run
APPLESCRIPT

sleep 2
diskutil mountDisk "$disk" >/dev/null || true
diskutil mount "${disk}s3" >/dev/null || true

mount_point="$(diskutil info "${disk}s3" | awk -F': *' '/Mount Point/ {print $2}')"
if [[ -z "$mount_point" || ! -d "$mount_point" ]]; then
  echo "could not mount ${disk}s3 after flashing" >&2
  diskutil list "$disk" >&2 || true
  exit 1
fi

mkdir -p "$mount_point/boot"
if [[ -f "$mount_point/boot/knulli" && ! -f "$mount_point/boot/knulli.original" ]]; then
  mv "$mount_point/boot/knulli" "$mount_point/boot/knulli.original"
fi
cp "$payload" "$mount_point/boot/knulli"
if [[ -f "$bootlogo" ]]; then
  if [[ -f "$mount_point/bootlogo.bmp" && ! -f "$mount_point/bootlogo.original.bmp" ]]; then
    cp "$mount_point/bootlogo.bmp" "$mount_point/bootlogo.original.bmp"
  fi
  cp "$bootlogo" "$mount_point/bootlogo.bmp"
fi
if [[ -d "$sample_dir" ]]; then
  rm -rf "$mount_point/music"
  mkdir -p "$mount_point/music"
  MUSICOS_MOUNT_POINT="$mount_point" \
  MUSICOS_SAMPLE_DIR="$sample_dir" \
  MUSICOS_SAMPLE_LIMIT="$sample_limit" \
  MUSICOS_AUDIO_FORMAT="$audio_format" \
  MUSICOS_OGG_QUALITY="$ogg_quality" \
  python3 - <<'PY'
import os
import shutil
import subprocess
from pathlib import Path

mount = Path(os.environ["MUSICOS_MOUNT_POINT"])
sample_dir = Path(os.environ["MUSICOS_SAMPLE_DIR"])
limit = int(os.environ.get("MUSICOS_SAMPLE_LIMIT", "0"))
audio_format = os.environ.get("MUSICOS_AUDIO_FORMAT", "ogg").lower()
ogg_quality = os.environ.get("MUSICOS_OGG_QUALITY", "5")
supported = {".mp3", ".flac", ".wav", ".ogg", ".m4a", ".aac"}
ffmpeg = shutil.which("ffmpeg")
files = sorted(
    (path for path in sample_dir.rglob("*") if path.is_file() and path.suffix.lower() in supported),
    key=lambda path: str(path.relative_to(sample_dir)).lower(),
)
copied = 0
for src in files:
    if limit and copied >= limit:
        break
    rel = src.relative_to(sample_dir)
    ext = src.suffix.lower()
    dest = mount / "music" / rel
    if audio_format == "ogg" and ffmpeg and ext != ".ogg":
        dest = dest.with_suffix(".ogg")
        dest.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            [ffmpeg, "-v", "error", "-y", "-i", str(src), "-map", "0:a:0", "-codec:a", "libvorbis", "-q:a", ogg_quality, str(dest)],
            check=True,
        )
    else:
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
    copied += 1
print(f"Copied {copied} sample song(s) to {mount / 'music'}")
PY
fi
sync
diskutil eject "$disk"
echo "Done. $disk now uses the RG35XX/KNULLI boot chain with the MusicOS squashfs payload."
