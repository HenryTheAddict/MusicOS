#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rootfs_tar="$repo_root/out/rg35xx/images/rootfs.tar"
out_root="$repo_root/out/rg35xx/knulli-ui-root"
payload="$repo_root/out/rg35xx/images/musicos-knulli-ui.squashfs"

if [[ -f "$rootfs_tar" ]]; then
  rm -rf "$out_root"
  mkdir -p "$out_root"
  tar -xf "$rootfs_tar" -C "$out_root"
elif [[ -d "$out_root/usr/lib" ]]; then
  echo "Using pre-extracted KNULLI rootfs at $out_root"
else
  echo "missing rootfs tar and extracted KNULLI rootfs: $rootfs_tar" >&2
  exit 1
fi
if [[ ! -e "$out_root/bin/sh" ]]; then
  ln -s ash "$out_root/bin/sh"
fi
mkdir -p \
  "$out_root/boot" \
  "$out_root/overlay" \
  "$out_root/run" \
  "$out_root/tmp" \
  "$out_root/proc" \
  "$out_root/sys" \
  "$out_root/dev" \
  "$out_root/usr/share/musicos" \
  "$out_root/var/lib/musicos" \
  "$out_root/var/log" \
  "$out_root/music" \
  "$out_root/etc"

echo "Building compact UI payload. Music is copied to the FAT partition by flash-knulli-music-os.sh."
rm -rf "$out_root/usr/share/musicos/assets"
cp -R "$repo_root/assets" "$out_root/usr/share/musicos/assets"
find "$out_root/usr/share/musicos/assets" -name '.DS_Store' -delete

cat >"$out_root/etc/asound.conf" <<'SH'
pcm.!default {
  type plug
  slave.pcm "hw:0,0"
}

ctl.!default {
  type hw
  card 0
}
SH

if [[ -x "$out_root/sbin/init" ]]; then
  mv "$out_root/sbin/init" "$out_root/sbin/musicos-init"
fi
fresh_init="${MUSICOS_INIT_BIN:-}"
if [[ -n "$fresh_init" && -x "$fresh_init" ]]; then
  cp "$fresh_init" "$out_root/sbin/musicos-init"
fi

cat >"$out_root/sbin/init" <<'SH'
#!/bin/sh
PATH=/sbin:/bin:/usr/sbin:/usr/bin
export PATH
export MUSICOS_RESOLUTION=640x480
export MUSICOS_MEDIA_DB=/boot/musicos/media.sqlite3
export MUSICOS_SETTINGS_PATH=/boot/musicos/settings.json
export MUSICOS_ASSETS_DIR=/usr/share/musicos/assets
export MUSICOS_TARGET_FPS=30
export MUSICOS_AUDIO_BACKEND=alsa
export MUSICOS_ALSA_DEVICE=${MUSICOS_ALSA_DEVICE:-plughw:0,0}
export ALSA_CONFIG_PATH=/usr/share/alsa/alsa.conf
export HOME=/root

LOG_TMP=/tmp/musicos-boot.log
LOG_BOOT=/boot/musicos-boot.log

stamp() {
  if [ -r /proc/uptime ]; then
    read up _ </proc/uptime
    printf '[%s] %s\n' "$up" "$*" >>"$LOG_TMP"
  else
    printf '[no-proc] %s\n' "$*" >>"$LOG_TMP"
  fi
  printf 'MusicOS UI: %s\n' "$*" >/dev/console 2>/dev/null || true
}

persist_log() {
  if mountpoint -q /boot 2>/dev/null; then
    mount -o remount,rw /boot 2>/dev/null || true
    cp "$LOG_TMP" "$LOG_BOOT" 2>/dev/null || true
    sync
  fi
}

mount -o remount,rw /boot 2>/dev/null || true
mkdir -p /tmp /run /root /var/lib/musicos /var/log /proc /sys /dev /boot/musicos
: >"$LOG_TMP"
stamp "ui wrapper entered"

mountpoint -q /proc 2>/dev/null || mount -t proc proc /proc 2>/dev/null || true
mountpoint -q /sys 2>/dev/null || mount -t sysfs sysfs /sys 2>/dev/null || true
mountpoint -q /run 2>/dev/null || mount -t tmpfs tmpfs /run 2>/dev/null || true

stamp "cmdline: $(cat /proc/cmdline 2>/dev/null)"
stamp "fb: $(ls -1 /dev/fb* 2>/dev/null | tr '\n' ' ')"
if [ -r /sys/class/graphics/fb0/virtual_size ]; then
  stamp "fb0 virtual_size=$(cat /sys/class/graphics/fb0/virtual_size 2>/dev/null)"
fi
if [ -r /sys/class/graphics/fb0/stride ]; then
  stamp "fb0 stride=$(cat /sys/class/graphics/fb0/stride 2>/dev/null)"
fi
stamp "input: $(ls -1 /dev/input/event* 2>/dev/null | tr '\n' ' ')"
stamp "snd: $(ls -1 /dev/snd/* 2>/dev/null | tr '\n' ' ')"
if [ -r /proc/asound/cards ]; then
  stamp "alsa_cards: $(cat /proc/asound/cards 2>/dev/null | tr '\n' ' ')"
fi
if [ -r /proc/asound/pcm ]; then
  stamp "alsa_pcm: $(cat /proc/asound/pcm 2>/dev/null | tr '\n' ' ')"
fi
if command -v amixer >/dev/null 2>&1; then
  stamp "amixer_controls: $(amixer scontrols 2>/dev/null | tr '\n' ' ')"
  for control in Master Headphone Speaker PCM "Line Out" DAC Digital Playback; do
    amixer sset "$control" 85% unmute >/dev/null 2>&1 && stamp "mixer_init: $control ok" || true
  done
fi
if command -v aplay >/dev/null 2>&1; then
  stamp "aplay_devices: $(aplay -l 2>/dev/null | tr '\n' ' ')"
fi
if [ -d /boot/music ]; then
  mkdir -p /music
  if [ ! -e /music/Samples ]; then
    ln -s /boot/music /music/Samples 2>/dev/null || true
  fi
fi
stamp "samples_boot=$(find /boot/music -type f 2>/dev/null | wc -l)"
stamp "samples_music=$(find /music -type f 2>/dev/null | wc -l)"
persist_log

stamp "launching /sbin/musicos-init"
/sbin/musicos-init >>"$LOG_TMP" 2>&1
rc=$?
stamp "musicos-init exited rc=$rc"
persist_log

while true; do sleep 5; done
SH
chmod +x "$out_root/sbin/init"

mkdir -p "$(dirname "$payload")"
rm -f "$payload"
mksquashfs "$out_root" "$payload" -comp gzip -noappend -all-root
unsquashfs -ll "$payload" | grep -E 'sbin/init|sbin/musicos-init|libasound|/music' | head -60
ls -lh "$payload"
