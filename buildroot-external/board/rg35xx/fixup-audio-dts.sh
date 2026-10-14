#!/usr/bin/env sh
set -eu

kernel_dir="${1:-}"
dts="$kernel_dir/arch/arm64/boot/dts/allwinner/sun50i-h700-anbernic-rg35xx-2024.dts"

if [ ! -f "$dts" ]; then
  exit 0
fi

if grep -q '"Speaker"[[:space:]]*,[[:space:]]*"LINEOUT"' "$dts" \
  && grep -q 'allwinner,pa-gpios[[:space:]]*=[[:space:]]*<&pio[[:space:]]*8[[:space:]]*5[[:space:]]*GPIO_ACTIVE_HIGH>' "$dts"; then
  printf 'MusicOS RG35XX audio DTS fixup: upstream route and PI5 amp GPIO already present\n'
  exit 0
fi

cat >>"$dts" <<'DTS'

/* MusicOS RG35XX 2024 audio fixup: keep this narrow and drop it once the
 * selected Linux tree carries the H700 codec route and PI5 speaker amp GPIO.
 */
&codec {
	allwinner,audio-routing = "Speaker", "LINEOUT",
				  "Headphone", "LINEOUT";
	allwinner,pa-gpios = <&pio 8 5 GPIO_ACTIVE_HIGH>; /* PI5 */
};
DTS

printf 'MusicOS RG35XX audio DTS fixup: added LINEOUT route and PI5 amp GPIO\n'
