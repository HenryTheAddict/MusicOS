# Visualizer reading, focus and color correction — 2026-11-03

The screenshot's blue rectangle was focus on the whole Lyrics region. The outer region is no longer a Tab stop. Its scroll area retains keyboard access with a compact filled reading hint instead of a viewport outline; the active timed cue remains reachable, with an underline focus treatment. Context menus, scrolling, Space handling and the supplied Queue → Lyrics → Normal morph remain available. The cycle tooltip now names the current view and next action clearly.

Visualizer lyric sizing is bounded by width and height, with a 3.5 rem ceiling. It remains larger than ordinary list text, while long refrains wrap less aggressively and leave space for the next cue. Upcoming cues are quieter and lightly defocused only while following. Manual reading restores clarity. No text scaling or transport relocation is added.

Album tinting previously mapped equal-brightness pixels to one color, flattening different preset regions into purple or brown. The native final-output shader now retains hue-region distinctions and fits chroma around the original luminance rather than clipping channels. Black, white, SDR bounds and exact original output when disabled remain preserved. Playback icons and seek colors use neutral soft shading instead of the unrelated fixed blue.

Verification: UI build passed; **71 files / 221 UI tests passed**; **16 browser checks passed**, including keyboard focus and 48 size/scale/theme combinations. A real Metal readback checks that equal-luminance red and green regions stay distinct after palette mapping, that luminance is retained within one 8-bit level, and that black/white, invalid-palette fallback and exact disabled output remain correct.

The silent native fixture was inspected with actual Metal output, keyboard Shift+Tab focus and Album colors enabled. There was no viewport outline; the compact Lyrics reading hint appeared, the next cue was subdued, and separate green/gold/blue artwork colors remained visible. It does not use the user's audio or profile.

Signed executable SHA-256: `2c9df52ac352f95006a1dc8797bcfdc63ee3a2b9f07939041a3e208f1392749d`. Delivery preserves the latest profile, collections, previous app and protected helper/runtime/model hashes. This focused repair does not close the broader Automix listening, Apple authentication or cross-platform gates.
