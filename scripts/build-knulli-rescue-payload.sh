#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src_root="$repo_root/third_party/knulli/ramdisk-root"
out_root="$repo_root/out/rg35xx/knulli-rescue-root"
payload="$repo_root/out/rg35xx/images/musicos-knulli-rescue.squashfs"
bootlogo="$repo_root/out/rg35xx/images/musicos-bootlogo.bmp"

if [[ ! -d "$src_root" ]]; then
  echo "missing extracted KNULLI ramdisk root: $src_root" >&2
  exit 1
fi

rm -rf "$out_root"
mkdir -p "$out_root"
rsync -a "$src_root/" "$out_root/"
mkdir -p \
  "$out_root/boot" \
  "$out_root/overlay" \
  "$out_root/run" \
  "$out_root/tmp" \
  "$out_root/var/lib/musicos" \
  "$out_root/usr/share/musicos" \
  "$out_root/music/Samples"

sample_dir="${MUSICOS_SAMPLE_DIR:-$repo_root/samples/tanger}"
if [[ ! -d "$sample_dir" ]]; then
  sample_dir="$repo_root/out/sample-songs"
fi

sample_count="$(find "$sample_dir" -type f \( -iname '*.mp3' -o -iname '*.flac' -o -iname '*.wav' -o -iname '*.ogg' -o -iname '*.m4a' -o -iname '*.aac' \) | wc -l | tr -d ' ')"
if [[ "$sample_count" == "0" && -z "${MUSICOS_SAMPLE_DIR:-}" && "$sample_dir" == "$repo_root/samples/tanger" ]]; then
  sample_dir="$repo_root/out/sample-songs"
  sample_count="$(find "$sample_dir" -type f \( -iname '*.mp3' -o -iname '*.flac' -o -iname '*.wav' -o -iname '*.ogg' -o -iname '*.m4a' -o -iname '*.aac' \) | wc -l | tr -d ' ')"
fi
if [[ "$sample_count" == "0" ]]; then
  echo "no supported audio files found in sample directory: $sample_dir" >&2
  exit 1
fi

while IFS= read -r sample; do
  cp "$sample" "$out_root/music/Samples/"
done < <(find "$sample_dir" -type f \( -iname '*.mp3' -o -iname '*.flac' -o -iname '*.wav' -o -iname '*.ogg' -o -iname '*.m4a' -o -iname '*.aac' \) | sort)
echo "Using $sample_count sample song(s) from $sample_dir"

python3 - <<'PY'
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

root = Path("out/rg35xx/knulli-rescue-root/usr/share/musicos")

def font(size: int):
    candidates = [
        "assets/fonts/RasterGothic.ttf",
        "assets/fonts/Raster Gothic.ttf",
        "assets/fonts/raster-gothic.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ]
    for candidate in candidates:
        path = Path(candidate)
        if path.exists():
            try:
                return ImageFont.truetype(str(path), size)
            except Exception:
                pass
    return ImageFont.load_default()

def gradient_image(width: int, height: int, status: str | None = None, pct: int | None = None) -> Image.Image:
    img = Image.new("RGB", (width, height))
    px = img.load()
    for y in range(height):
        g = 68 + min(112, (y * 112) // max(1, height - 1))
        for x in range(width):
            px[x, y] = (0, g, 255)
    draw = ImageDraw.Draw(img)
    title_font = font(max(24, width // 11))
    body_font = font(max(14, width // 26))
    small_font = font(max(10, width // 42))
    title = "loading"
    bbox = draw.textbbox((0, 0), title, font=title_font)
    draw.text(((width - (bbox[2] - bbox[0])) // 2, height * 0.33), title, fill=(255, 255, 255), font=title_font)
    if status:
        bbox = draw.textbbox((0, 0), status, font=body_font)
        draw.text(((width - (bbox[2] - bbox[0])) // 2, height * 0.50), status, fill=(235, 245, 255), font=body_font)
    if pct is not None:
        bar_w = int(width * 0.58)
        bar_h = max(8, height // 54)
        x0 = (width - bar_w) // 2
        y0 = int(height * 0.62)
        draw.rounded_rectangle((x0, y0, x0 + bar_w, y0 + bar_h), radius=bar_h // 2, fill=(170, 210, 255))
        draw.rounded_rectangle((x0, y0, x0 + int(bar_w * pct / 100), y0 + bar_h), radius=bar_h // 2, fill=(255, 255, 255))
        text = f"{pct}%"
        bbox = draw.textbbox((0, 0), text, font=small_font)
        draw.text(((width - (bbox[2] - bbox[0])) // 2, y0 + bar_h + 10), text, fill=(245, 250, 255), font=small_font)
    return img

def write_frame(path: str, width: int, height: int, status: str, pct: int) -> None:
    img = gradient_image(width, height, status, pct).convert("RGBA")
    rgba = img.tobytes()
    buf = bytearray(len(rgba))
    for i in range(0, len(rgba), 4):
        r, g, b, a = rgba[i:i+4]
        buf[i:i+4] = bytes((b, g, r, 0))
    Path(path).write_bytes(buf)

Path("out/rg35xx/images").mkdir(parents=True, exist_ok=True)
gradient_image(640, 480, "starting MusicOS", 8).save("out/rg35xx/images/musicos-bootlogo.bmp", format="BMP")

def write_gradient(path: str, width: int, height: int) -> None:
    buf = bytearray(width * height * 4)
    for y in range(height):
        for x in range(width):
            i = (y * width + x) * 4
            # Most fbdev setups here are BGRA/XRGB-ish. This produces MusicOS blue.
            buf[i + 0] = 255
            buf[i + 1] = 68 + min(112, (y * 112) // max(1, height - 1))
            buf[i + 2] = 0
            buf[i + 3] = 0
    Path(path).write_bytes(buf)

write_gradient(str(root / "rescue-640x480.bgra"), 640, 480)
write_gradient(str(root / "rescue-1280x960.bgra"), 1280, 960)
stages = [
    ("stage-10", "kernel handed off", 10),
    ("stage-25", "mounting runtime", 25),
    ("stage-45", "probing display", 45),
    ("stage-65", "starting USB logs", 65),
    ("stage-85", "checking media", 85),
    ("stage-100", "ready", 100),
]
for name, status, pct in stages:
    write_frame(str(root / f"{name}-640x480.bgra"), 640, 480, status, pct)
    write_frame(str(root / f"{name}-1280x960.bgra"), 1280, 960, status, pct)
PY

rm -f "$out_root/sbin/init"
cat >"$out_root/sbin/init" <<'SH'
#!/bin/ash
PATH=/sbin:/bin:/usr/sbin:/usr/bin

LOG_TMP=/tmp/musicos-boot.log
LOG_BOOT=/boot/musicos-boot.log
USB_LOG=/dev/ttyGS0
FB_SIZE=640
FB_FRAME_SUFFIX=640x480

stamp() {
  if [ -r /proc/uptime ]; then
    read up _ </proc/uptime
    printf '[%s] %s\n' "$up" "$*" >>"$LOG_TMP"
  else
    printf '[no-proc] %s\n' "$*" >>"$LOG_TMP"
  fi
  printf 'MusicOS: %s\n' "$*" >/dev/console 2>/dev/null
  if [ -e "$USB_LOG" ]; then
    printf 'MusicOS: %s\n' "$*" >"$USB_LOG" 2>/dev/null || true
  fi
}

draw_stage() {
  stage="$1"
  frame="/usr/share/musicos/${stage}-${FB_FRAME_SUFFIX}.bgra"
  if [ -e /dev/fb0 ] && [ -f "$frame" ]; then
    cat "$frame" >/dev/fb0 2>>"$LOG_TMP" || true
  fi
}

persist_log() {
  if mountpoint -q /boot 2>/dev/null; then
    mount -o remount,rw /boot 2>/dev/null || true
    cp "$LOG_TMP" "$LOG_BOOT" 2>/dev/null || true
    sync
  fi
}

mkdir -p /tmp /run /var/lib/musicos
: >"$LOG_TMP"
stamp "rescue pid1 entered"
draw_stage stage-10
stamp "cmdline: $(cat /proc/cmdline 2>/dev/null)"

# KNULLI's initramfs already mounted and moved /proc, /sys, /dev, and /boot.
# Avoid remounting /dev during the critical path; that was slow and could hide fb0.
mountpoint -q /proc 2>/dev/null || mount -t proc proc /proc 2>/dev/null || true
mountpoint -q /sys 2>/dev/null || mount -t sysfs sysfs /sys 2>/dev/null || true
mountpoint -q /run 2>/dev/null || mount -t tmpfs tmpfs /run 2>/dev/null || true

stamp "mounts ready"
draw_stage stage-25
stamp "fb devices: $(ls -1 /dev/fb* 2>/dev/null | tr '\n' ' ')"
if [ -r /sys/class/graphics/fb0/virtual_size ]; then
  fb_virtual_size="$(cat /sys/class/graphics/fb0/virtual_size 2>/dev/null)"
  stamp "fb0 virtual_size=${fb_virtual_size}"
  case "$fb_virtual_size" in
    1280,960|1280x960)
      FB_FRAME_SUFFIX=1280x960
      ;;
  esac
fi
if [ -r /sys/class/graphics/fb0/stride ]; then
  stamp "fb0 stride=$(cat /sys/class/graphics/fb0/stride 2>/dev/null)"
fi

if [ -e /dev/fb0 ]; then
  draw_stage stage-45
  size="$(cat /sys/class/graphics/fb0/virtual_size 2>/dev/null || true)"
  case "$size" in
    1280,960|1280x960)
      stamp "drawing 1280x960 rescue frame"
      cat /usr/share/musicos/rescue-1280x960.bgra >/dev/fb0 2>>"$LOG_TMP" || stamp "framebuffer write failed"
      ;;
    *)
      stamp "drawing 640x480 rescue frame"
      cat /usr/share/musicos/rescue-640x480.bgra >/dev/fb0 2>>"$LOG_TMP" || stamp "framebuffer write failed"
      ;;
  esac
else
  stamp "no /dev/fb0 found"
fi

draw_stage stage-65
if [ -d /sys/kernel/config/usb_gadget ]; then
  stamp "usb configfs available"
  mkdir -p /sys/kernel/config/usb_gadget/musicos 2>/dev/null || true
  if [ -d /sys/kernel/config/usb_gadget/musicos ]; then
    cd /sys/kernel/config/usb_gadget/musicos 2>/dev/null || true
    echo 0x1d6b > idVendor 2>/dev/null || true
    echo 0x0104 > idProduct 2>/dev/null || true
    mkdir -p strings/0x409 configs/c.1/strings/0x409 functions/acm.usb0 2>/dev/null || true
    echo MusicOS > strings/0x409/manufacturer 2>/dev/null || true
    echo RG35XX-2024 > strings/0x409/product 2>/dev/null || true
    echo musicos-bootlog > strings/0x409/serialnumber 2>/dev/null || true
    echo "MusicOS Logs" > configs/c.1/strings/0x409/configuration 2>/dev/null || true
    ln -s functions/acm.usb0 configs/c.1/ 2>/dev/null || true
    udc="$(ls /sys/class/udc 2>/dev/null | head -1)"
    if [ -n "$udc" ]; then
      echo "$udc" > UDC 2>/dev/null || true
      stamp "usb serial logging requested on ttyGS0 via $udc"
    else
      stamp "usb serial unavailable: no UDC"
    fi
  fi
else
  stamp "usb configfs unavailable"
fi

draw_stage stage-85
stamp "sample_count=$(find /music/Samples -type f 2>/dev/null | wc -l)"
persist_log
draw_stage stage-100
stamp "rescue startup complete"
persist_log

while true; do
  sleep 5
  persist_log
done
SH
chmod +x "$out_root/sbin/init"

rm -f "$payload"
mksquashfs "$out_root" "$payload" -comp gzip -noappend -all-root
unsquashfs -ll "$payload" | grep -E 'sbin/init|rescue-(640|1280)|music/Samples' | head -40
ls -lh "$payload"
