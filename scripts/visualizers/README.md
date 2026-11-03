# Curated native visualizer validation

The shipped renderer executes Rust and original GPU programs only. The JavaScript reference below is an isolated developer fixture, never bundled or used as a fallback.

1. Prepare a separate `out/visualizer-reference/butterchurn` checkout at the exact `engine_reference_revision` in `assets/visualizers/manifest.json`. Install that reference's development dependencies.
2. For deterministic comparison, instrument both `PresetEquationRunner` and `PresetEquationRunnerWASM` immediately before their `rand_start` assignment: `window.__musicosInitialRandomState = Array.from(this.rng.getRNG().state);`. This records (without changing) the random stream after the reference's blank-preset/transition initialization. Its deterministic JavaScript `window.rand` wrapper may be changed to continuous normal-renderer semantics; the curated sources use the original WASM equation runner and its continuous `rand(x)` shim.
3. Run the reference's `npm run dev-build`. Copy `scripts/visualizers/reference.html` into `out/visualizer-reference`. Serve that directory on **127.0.0.1:17022**. Do not expose this development fixture externally.
4. Run `node scripts/visualizers/reference.mjs`. It saves deterministic 640×480, 180-frame images, main variables, original waveform/spectrum/band inputs and scripting measurements. `FRAMES` and `REFERENCE_OUTPUT` select shorter captures and a separate output directory.
5. Run `REFERENCE_AUDIO=out/visualizer-reference/butterchurn-renders cargo run -p musicos-visualizer --release --example render-check -- out/visualizer-reference/rust`.
6. Run the `pixel-validation`, `gpu-equation-benchmark`, and `gpu-full-benchmark` examples. The first compares the actual GPU mesh against the independent f64 pixel/warp reference at three aspect ratios. The others report their explicitly named CPU/GPU scopes, excluding the first ten warmup frames.

The Rust reference-state constructor aligns the initial random state; it does not substitute reference images, main values, pixel equations, shaders, shapes, or waveform geometry. Random source ordering matters for these feedback systems. Compare first frames and buildup as well as long-running images: repeated GPU filtering and chaotic feedback can amplify small rasterization differences.

The surface-proof Tauri example is isolated from AppState, accounts, library and audio output. Its optional visualizer UI fixture uses a separate development server on 127.0.0.1:1422. Browser mocks establish layout/keyboard behavior only, never Metal rendering, physical display pacing, audio synchronization, or installed-build acceptance.
