# musicOS credits and licenses

musicOS uses Figtree and the third-party software and artwork listed below. Friendly visualizer names do not replace the original authors’ attribution.

## Visualizers

The curated native ports use the selected Butterchurn preset sources, pinned to revision `c10e2616762f262de898e5d4c1f162a3e01340c1`. The reference engine is pinned to `fbac2f6bab62fd9c6a50ebbeb29359c5eb05903e`.

- [Butterchurn](https://github.com/jberg/butterchurn), copyright 2013–2018 Jordan Berg, MIT.
- [Butterchurn presets](https://github.com/jberg/butterchurn-presets), copyright 2013–2018 Jordan Berg and the original preset authors, MIT.
- [MilkDrop](https://www.geisswerks.com/milkdrop/) and its visualizer design: Ryan Geiss. Butterchurn and these ports build on the MilkDrop preset model.

Complete upstream licenses are in `third_party/licenses/butterchurn`. Original source, checksums, aliases, and port notes are in `assets/visualizers/manifest.json`.

| musicOS name | Original preset | Authors |
| --- | --- | --- |
| boxes | suksma - pus on my cat - j4+ - cubby | suksma, j4+, cubby |
| oil puddle | undulate harque - shit broo | undulate harque |
| carbonated | Zylot - In death there is life (Geiss Layered Mix) (Jelly) | Zylot; Geiss (layered mix) |
| watercolor | $$$ Royal - Mashup (441) | $$$ Royal |
| cells | $$$ Royal - Mashup (431) | $$$ Royal |
| tweaker | $$$ Royal - Mashup (337) | $$$ Royal |
| Universe | $$$ Royal - Mashup (307) | $$$ Royal |
| mold | flexi - borderline imagery | flexi |
| bazinga | flexi - bouncing balls [double mindblob gastrointestinal mix] | flexi |
| infinite | Flexi, fishbrain + Martin - witchery | Flexi, fishbrain + Martin |
| outfinate | Flexi, fishbrain, Geiss + Martin - tokamak witchery | Flexi, fishbrain, Geiss + Martin |
| paint stir | Flexi, martin + geiss - dedicated to the sherwin maxawow | Flexi, martin + geiss |
| mucus | martin - mucus cervix | martin |

## Native renderer

The Rust visualizer uses wgpu / wgpu-core / wgpu-hal and Naga (wgpu contributors), Metal (Apple), bytemuck (Lokathor and contributors), pollster (Joshua Barretto and contributors), raw-window-handle (contributors), and the Rust dependencies recorded in `assets/visualizers/renderer-dependencies.json`. Complete bundled dependency notices are in `third_party/licenses/visualizer`.

The renderer does not execute preset JavaScript. The native ports retain the selected original warp/composite shader bodies and implement the equation, feedback, shape, and waveform paths required by this catalog. The manifest identifies port modifications; validation status is recorded separately from these credits.

## Typography

Figtree, Erik D. Kennedy and contributors, SIL Open Font License 1.1. See the bundled font license under the existing typography assets.

## Other application dependencies

Additional third-party packages retain their package licenses and existing application attributions. Song contributor credits in `credits.ts` are separate from these software credits.

## Controller input and vibration

Native controller input and force feedback use [Gilrs 0.11.2](https://gitlab.com/gilrs-project/gilrs) and gilrs-core 0.6.8, by the Gilrs contributors, under MIT OR Apache-2.0. Linux device discovery uses libudev-sys 0.1.4 (MIT), and controller storage uses vec_map 0.8.2 (MIT OR Apache-2.0). Complete supplied notices are bundled in `third_party/licenses/controller`. Gilrs bundles the SDL Game Controller Database, under its zlib license; its notice is included alongside the controller licenses. Vibration availability depends on hardware and the operating-system driver.

The Linux build bundles the Microsoft ONNX Runtime 1.31.0 CPU runtime under MIT with its third-party notices, alongside the existing model. Runtime source and checksums are recorded by `scripts/linux/prepare-runtime.py`; complete notices are in `third_party/licenses/onnxruntime-MIT.txt` and `onnxruntime-ThirdPartyNotices.txt`.

## iPhone companion

The iPhone companion uses Tauri 2 (MIT or Apache-2.0), SwiftRs (MIT or Apache-2.0), Symphonia 0.5.5 by the Symphonia contributors (MPL-2.0), and Hound 3.5.1 by Ruud van Asseldonk (Apache-2.0). Supplied notices are in `third_party/licenses/companion`. Symphonia is unmodified; its corresponding source is available at https://github.com/pdeljanov/Symphonia/tree/v0.5.5. musicOS’s decoder integration is in `ios/src-tauri/src/decoder.rs`.

The companion’s SDR artwork extraction reuses the desktop color algorithm and image 0.24.9 (MIT or Apache-2.0), with its PNG/JPEG/WebP decoding dependencies. Their supplied complete notices are bundled under `third_party/licenses/companion`.
