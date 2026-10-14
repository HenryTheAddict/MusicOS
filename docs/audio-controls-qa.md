# Audio controls delivery — 13 October 2026

## Changes

Quick Audio uses a compact bento arrangement with a slim, thumb-free volume rail, thin EQ sliders, a smooth frequency-response curve, paired speed/pitch controls, and optional damped stereo reverb. The larger first preview was rejected and replaced; it was never loaded into the running app. Settings' adaptive selected-category surface now uses an explicit readable foreground/background pair.

Speed and pitch updates no longer reopen or seek the decoder. Worker parameters retain the latest target, smooth changes once per 512-source-frame block, and keep Signalsmith history warm. A latency-aligned dry path and 20 ms wet/dry crossfade avoid abrupt bypass changes. The streaming ring is bounded to 4,000 stereo frames (83.3 ms at 48 kHz). Filter calculation, decoding, stretch work, and buffer allocation remain outside the output callback. Each output frame carries the listening revision; playing-state acknowledgements follow rendered frames. Paused controls acknowledge the accepted target without resuming playback.

Preference writes coalesce pending changes while preserving the last complete preference snapshot and atomic native persistence. EQ headroom, output limiting, and user/DJ tempo separation remain in place; speed/pitch edits cancel and replan DJ transitions. Reverb buffers are preallocated, pause freezes processing, seek clears old tails, and reverb defaults to zero/dry.

Lossless sources are not transcoded through a lossy codec. The existing stereo 48 kHz desktop output format remains unchanged. EQ, reverb, speed, and pitch processing alter samples and are not bit-perfect; flat, normal-speed dry playback bypasses those processors. This is not a claim of source-rate-exclusive output.

## Verification

- UI build and 63 frontend unit tests passed, including filter response at band centers and finite curved paths.
- 60 desktop native tests passed, including exact dry EQ bypass, reverb decay/bypass, parameter reversals, source playback, and existing mixer/tempo tests.
- 96 Tauri tests passed.
- Five browser tests passed: compact/wide controls, linked/independent parameters, persistence/reset, dry-by-default reverb, invisible volume thumb, Settings pink contrast, and a 40-case window/scale/palette matrix.
- Render inspection used small/wide windows, 80/100/125/150 percent scales, and blue/light/dark/vivid/adaptive surfaces. Browser geometry checks are separate from design approval.
- Native decoder/output probe used a read-only source lookup and a disposable database at zero output volume. It did not edit the user's library or preferences.
- Initial parameter acknowledgement: 108.22 ms. Five subsequent speed changes: 116.19, 117.58, 109.55, 110.17, 109.72 ms. Source position never moved backward during changes. Measured settled source rate: 1.4603 for a 1.5 target, within the existing 0.12 test tolerance. Pause froze source time; independent controls and seek passed.

## Acceptance limits

The native probe was muted. Audible rapid-change smoothness, reverb character, musical DJ acceptance, system media commands, Sparklist blind listening, and measured 120 Hz presentation remain open. No claim of instant/zero-latency control or bit-perfect processed output is made. Windows/Linux remain unverified. No model assets or source music were changed.

Build identity and load result are recorded below after signing.

## Signed package and load

- Package: `target/release/bundle/macos/MusicOS.app`.
- Executable SHA-256: `3921ae46dede1db9b1595990a01c93533c536dd979f61ff265a1600734e24492`.
- Ad-hoc deep signing and strict deep verification passed.
- Executable built at 21:55:50 EDT; running process began at 21:56:02 EDT from that package. The updated native UI exposed the frequency-response chart and reverb controls. Playback was paused before the update point. The user subsequently interacted with the updated app; automation left their controls and selection alone.
- Embedded sidecar SHA-256 remains `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- Embedded ONNX runtime SHA-256 remains `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.
