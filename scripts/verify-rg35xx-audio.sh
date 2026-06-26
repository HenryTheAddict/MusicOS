#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build_dir="${1:-$root/out/rg35xx}"
config="$build_dir/.config"
kernel_dir="$(find "$build_dir/build" -maxdepth 1 -type d -name 'linux-*' 2>/dev/null | head -1 || true)"
dts="$kernel_dir/arch/arm64/boot/dts/allwinner/sun50i-h700-anbernic-rg35xx-2024.dts"

required_symbols=(
  BR2_PACKAGE_ALSA_LIB
  BR2_PACKAGE_ALSA_LIB_PCM
  BR2_PACKAGE_ALSA_UTILS
  BR2_PACKAGE_ALSA_UTILS_AMIXER
  BR2_PACKAGE_ALSA_UTILS_APLAY
)

kernel_symbols=(
  CONFIG_SOUND
  CONFIG_SND
  CONFIG_SND_ALSA
  CONFIG_SND_SOC
  CONFIG_SND_SIMPLE_CARD
  CONFIG_SND_SOC_SIMPLE_AMPLIFIER
  CONFIG_SND_SUNXI_SOC_AUDIO
  CONFIG_SND_SUN4I_CODEC
  CONFIG_SND_SUN8I_CODEC
  CONFIG_SND_SUN8I_CODEC_ANALOG
)

missing=0

check_symbol() {
  file="$1"
  symbol="$2"
  if ! grep -q "^${symbol}=y" "$file" 2>/dev/null; then
    echo "missing ${symbol} in ${file}" >&2
    missing=1
  fi
}

for symbol in "${required_symbols[@]}"; do
  check_symbol "$config" "$symbol"
done

kernel_config="$(find "$build_dir/build" -path '*/.config' -name .config 2>/dev/null | head -1 || true)"
if [[ -n "$kernel_config" ]]; then
  for symbol in "${kernel_symbols[@]}"; do
    check_symbol "$kernel_config" "$symbol"
  done
else
  echo "kernel .config not found under $build_dir/build; run after kernel configure/build" >&2
  missing=1
fi

if [[ -f "$dts" ]]; then
  grep -q '"Line Out"[[:space:]]*,[[:space:]]*"LINEOUT"' "$dts" || {
    echo "missing Line Out -> LINEOUT route in $dts" >&2
    missing=1
  }
  grep -q 'allwinner,pa-gpios[[:space:]]*=[[:space:]]*<&pio[[:space:]]*8[[:space:]]*5[[:space:]]*GPIO_ACTIVE_HIGH>' "$dts" || {
    echo "missing PI5 speaker amp GPIO in $dts" >&2
    missing=1
  }
else
  echo "RG35XX DTS not found yet under $kernel_dir" >&2
  missing=1
fi

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo "RG35XX audio config verified."
