# Reverb XY pad — 2026-10-14

The compact square in Quick Audio has a circular handle. Horizontal movement adjusts the existing wet/dry amount; vertical movement adjusts decay depth. Old preferences keep their saved amount and default to 50% depth, preserving the original room feedback. Reverb remains off by default.

Depth uses bounded, smoothed feedback (0.52–0.92) in the existing preallocated stereo delay network. It does not allocate buffers, reload decoders, modify EQ, or change DJ tempo. The playback event reports the mixer's acknowledged depth alongside the existing wet amount and listening revision. Off remains an exact dry bypass below the existing output safety limiter.

## Automated results

- Desktop core: 73 passed, including rendered shallow/deep tail comparison, finite output, dry bypass under rapid depth changes, old-preference defaults, and native wet/depth acknowledgement without decoder reload.
- Tauri: 96 passed.
- Frontend: 63 passed.
- Browser: 6 passed, including pointer capture outside the pad, both axes, keyboard steps, persistence, fixed square dimensions, off preserving depth, and Audio geometry at 720×480 / 1280×800 across four scales and five palettes.
- Production UI build passed.

The browser render was inspected. Automated output tests establish an actual decay difference; they do not constitute subjective listening acceptance. Existing audible DJmode, macOS remote-control, and Sparklist listening gates remain separate. Windows/Linux are unverified.

## Packaged delivery

Build identity and native verification are recorded below after packaging. The embedded runtime and source music are preserved; reload is allowed only while playback is paused or stopped.

- Executable SHA-256: `4c6d2b85c7597fe5f1da1397ec89ddd808f95af5dd295d6272c8cc3e1b94664f`.
- Deep ad-hoc signature and strict verification passed.
- Embedded sidecar and ONNX runtime hashes match the previous build.
- Playback was confirmed paused before reload.
- Native app loaded this executable at 17:06:39 EDT. Solitude remained paused at 1:17; 443 songs and existing collections restored.
- Native pad click applied 50% wet / 50% depth. Arrow Up independently applied 51% depth; the footer changed from Applying to Applies to all songs after mixer acknowledgement.
- Restored reverb off and depth 50% after checking. Left Audio open to show the new pad. No music was started for this UI verification.

## Compact revision

Removed the visible directions and numeric readout, reduced the pad from 80 to 52 CSS pixels at 100% scale, reduced its handle, and removed the surrounding reverb tile. Hover help and screen-reader/keyboard directions remain available.

Extended maximum feedback from 0.92 to 0.965 for longer decay. Compensated delay-network excitation above the original room depth keeps its steady-state gain bounded. The default 50% depth retains the original feedback. Desktop core (73) and all six browser cases passed again; the updated render was inspected. Subjective listening acceptance remains outstanding.

Compact packaged executable SHA-256: `8373469aeb5354777cbb1c4db20e8aadd2644fd1f08a124b5b80e8203e871175`. Deep ad-hoc signature and strict verification passed; embedded sidecar/runtime hashes are unchanged. Reloaded while CURIOUS was paused at 1:55.

## Subtle wet-tail stereo spread

Added two fixed all-pass delay paths (211 and 337 frames at 48 kHz) to decorrelate the wet midpoint into a restrained side contribution (0.12). The contribution is equal and opposite on the two channels, preserving the original wet mono sum before limiting. Dry audio, decay settings, and the compact UI remain unchanged. The buffers are fixed and allocated before playback; seek/navigation clears them with the room tail, while pause freezes processing.

Desktop core: 75 tests passed. New tests verify that mono input produces a stereo reverb tail, mono fold-down is preserved within floating-point tolerance, added side energy stays bounded, and clearing leaves no stale spread tail. Existing dry-bypass, decay, output, and playback tests passed. Subjective listening acceptance remains unverified.

Stereo-spread packaged executable SHA-256: `2306aca5866a43b5afc6234183837df83bca329435b377f8f63eb6f0152123c0`. Deep ad-hoc signature and strict verification passed; embedded runtime hashes are unchanged. This build was not loaded: Voyager was playing at the final check, so the existing process was left uninterrupted.
