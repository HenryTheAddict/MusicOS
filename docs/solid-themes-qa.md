# Solid themes and launch fade — October 16, 2026

## Behavior

- The first resolved theme blends from the original blue over 1.4 seconds. Later theme changes use 350 ms. The theme remains usable while colors resolve; no startup delay or image processing is introduced into the animation loop.
- Registered inherited CSS color properties animate the shell, surfaces, accents, text, and page together. Portaled menus inherit the same colors. A single solid page layer replaces the repeated radial-gradient reveal. Resize does not remount or replay the theme animation.
- Reduced Motion disables color interpolation. The launch phase also completes when artwork is missing or a palette lookup has no result.
- Adaptive colors retain the dominant artwork hue with more chroma: nominal shell tint increases from 52% to 70%, and colored page surfaces use 38% artwork color rather than 25%. Shell strength is bounded by 4.5:1 text contrast. Neutral artwork remains neutral. Palette cache version changes to v6; canonical artwork is untouched.
- Decorative gradients are removed from panel backgrounds, Settings previews, collection controls, icon shading, search sheen, and slider thumbs. Supplied icon geometry and actual artwork remain intact. Slider progress remains two discrete solid regions; the existing text-overflow alpha mask is retained to avoid a hard clipping edge.

## Verification

- Production TypeScript/Vite build passed.
- 71 frontend unit tests passed, including preservation of SVG geometry/clipping references while removing exported gradient shading.
- Native provider tests verify dominant color, cache reuse, neutral artwork, and readable richer palettes across green, blue, red, yellow, purple, dark, and pale fixtures.
- Browser checks cover observable intermediate launch colors, stable colors after resizing, Reduced Motion, missing artwork, restoring original blue, solid collection controls and Sparklist sheets, and existing flat Settings behavior.
- Wide album and Sparklist screenshots inspected separately from assertions. Screens retain their existing layout and supplied artwork.

## Delivery

Signed package identity and native results are recorded after the build. Load only while playback is paused/stopped. This pass does not claim native frame-rate measurements or change playback processing.

### Delivered package

- Local ad-hoc signed Apple Silicon bundle: `target/release/bundle/macos/MusicOS.app`, 98.41 MiB.
- Executable SHA-256: `d047e3eca75272c3bdef042d9639fc2ad1c411c88c482a600ecca4824de935c4`.
- Deep/strict signature verification passed; source sidecar and ONNX runtime hashes remain unchanged.
- Loaded while Picture in My Mind was paused at 91.2 seconds. The paused song/position, 507-song library, 11 favorites, collections, and current preferences survived reload.
- Native screenshot inspected: stronger yellow shell/surfaces, solid collection control backgrounds, readable text, and unchanged library layout. Launch interpolation is verified by the browser's intermediate computed-color samples; native frame timing is not measured.
- Seven native provider tests, 71 frontend tests, and nine distinct focused browser checks passed. Browser checks include the final no-artwork completion case and existing theme-disable/Settings coverage.
- Logs: `/tmp/musicos-solid-build.log`, `/tmp/musicos-solid-unit.log`, `/tmp/musicos-solid-provider-tests.log`, `/tmp/musicos-solid-browser.log`, `/tmp/musicos-solid-final-browser.log`, `/tmp/musicos-solid-ready-browser.log`, `/tmp/musicos-solid-package.log`.
