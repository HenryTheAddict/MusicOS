# Grouped Settings and Sparklist ranking

## Changes

Settings uses compact category rows on wide windows and a wrapping category bar on small windows. Reading width is bounded, oversized bento tiles become stacked groups, switches have divided rows, and Audio analysis uses the shared themed full-recheck switch. Arrow keys support both directions; existing preferences and shortcuts are retained.

Sparklist generation v2 bounds artist and album diversity penalties. Previously those penalties grew with every selected song and could eventually outrank strongly relevant audio. Diversity still breaks similar-score ties; audio embeddings, seed/mood weights, explicit feedback, consent and model validation remain in place. Saved sequences are unchanged until explicit refresh.

Apple SHARP Gaussian splatting is not installed or exposed: [Apple's model license](https://github.com/apple-aiml-research/ml-sharp/blob/main/LICENSE_MODEL) limits the weights to scientific research and expressly excludes product development. The existing Depth Anything F32 setup remains in Appearance → Artwork motion. No model assets were downloaded, substituted or published in this change.

## Verification

- Production UI build passed.
- All 47 frontend tests passed; rescan test updated for the shared switch's accessible role.
- All six Sparklist engine checks passed, including a regression where 24 relevant same-artist songs must rank before an unrelated song.
- Three focused browser checks passed: Settings navigation, switch independence, depth setup discoverability, quality shortcut and image viewer across 720×480 and 1280×800. The grouped-settings check passed again after small-window layout correction.
- Rendered Settings screenshots inspected at both sizes. All-scale/palette native acceptance was not repeated.
- Existing model listening acceptance remains open. This is a ranking correction, not a model-quality acceptance result.

The running app was playing music during inspection; it was not restarted or paused for installation.

## Packaged build

Apple Silicon bundle: `target/release/bundle/macos/MusicOS.app`.
Executable SHA-256: `ac7c0b2eb82b83538743b4bd647965e5d0183615123962f8fc2d0cfc17e5ecf2`.
Production build and strict deep ad-hoc signature verification passed. Packaged native UI acceptance was not rerun because music was playing; the running session remains on the previous build until restart.

Embedded sidecar SHA-256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime SHA-256 remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
