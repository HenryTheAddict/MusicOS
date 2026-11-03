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
