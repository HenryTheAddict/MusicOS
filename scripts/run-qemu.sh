#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/env.sh"
OUT="${ROOT}/out/qemu/images"
KERNEL="${OUT}/Image"
ROOTFS="${OUT}/rootfs.ext4"
RESOLUTION="${MUSICOS_RESOLUTION:-1280x960}"
XRES="${RESOLUTION%x*}"
YRES="${RESOLUTION#*x}"
QEMU_NET="${MUSICOS_QEMU_NET:-sync}"
QEMU_HOST_PORT="${MUSICOS_QEMU_HOST_PORT:-18080}"

if [[ ! -f "${KERNEL}" || ! -f "${ROOTFS}" ]]; then
  echo "Missing ${KERNEL} or ${ROOTFS}; run scripts/build-qemu.sh first." >&2
  exit 1
fi

CMD=(
  qemu-system-aarch64
  -M virt \
  -cpu cortex-a53 \
  -smp 4 \
  -m 512M \
  -kernel "${KERNEL}" \
  -drive "file=${ROOTFS},if=none,format=raw,id=hd0" \
  -device virtio-blk-pci,drive=hd0 \
  -device "virtio-gpu-pci,xres=${XRES},yres=${YRES}" \
  -device virtio-keyboard-pci \
  -device virtio-tablet-pci \
  -device virtio-sound-pci,audiodev=snd0 \
  -audiodev coreaudio,id=snd0 \
  -display cocoa,show-cursor=on \
  -serial mon:stdio \
  -no-reboot
)

if [[ "${QEMU_NET}" == "sync" ]]; then
  CMD+=(
    -append "console=ttyAMA0 root=/dev/vda rw rootwait init=/sbin/init MUSICOS_RESOLUTION=${RESOLUTION} ip=10.0.2.15::10.0.2.2:255.255.255.0::eth0:off"
    -netdev "user,id=net0,hostfwd=tcp::${QEMU_HOST_PORT}-:8080"
    -device virtio-net-pci,netdev=net0
  )
  echo "QEMU sync API forwarded at http://127.0.0.1:${QEMU_HOST_PORT}"
else
  CMD+=(
    -append "console=ttyAMA0 root=/dev/vda rw rootwait init=/sbin/init MUSICOS_RESOLUTION=${RESOLUTION}"
    -net none
  )
fi

exec "${CMD[@]}"
