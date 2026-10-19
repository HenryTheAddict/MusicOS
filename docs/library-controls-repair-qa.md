# Audio, depth, and everyday controls — 2026-10-15

## Implementation

- Supplied Discord, LastFM, Play Next, Add to queue, depth, and DJmode Sketch SVG exports, with source layer IDs in the asset manifest. White export canvas removed; glyph geometry retained.
- Soft white collection Shuffle/Play surfaces with actual cover colors, solid readable glyphs, and restrained Sparklist surface overlays. Removed invented hue rotations and glyph-gradient animation.
- Compact playlist creation, separate artwork and multi-song pickers; existing staged Sparklist creation retained. Bounded pointer/overflow menus and manual-only membership picker. DJmode glyph in both queue presentations.
- Reverb wet excitation now rejects DC/very-low-frequency buildup; preallocated allpass diffusion makes the tail less sparse. Saved amounts, wet-only stereo spread, dry bypass, and listening revisions retained.
- Shared depth caches: 256 MiB prepared RGBA images, 64 MiB encoded responses, and 128 MiB active GPU textures. Visible resources pinned, nearby row prefetched, offscreen LRU eviction. Resize retains the compositor; transient failures retry, GPU loss reconstructs from RAM, disk maps remain authoritative. Native scroll behavior replaces synthetic stretch/blur.
- Adaptive saturation raised with a 4.5:1 text contrast bound. Canonical palette cache version advanced.
- Compact opt-in integration rows, guided credential setup, diagnostics in Details. Candidate-specific metadata ignoring and Restore persist transactionally. Ordinary folder removal soft-disables; relocation hashes matching files and preserves unmatched records and identities.

## Output evidence

Production mixer capture, real Moon Crystal FLAC excerpt, 384,000 source frames:

- Reverb 45% difference RMS: 0.0874056; tail energy: 8.25791.
- Dry output includes the pre-existing safety limiter; full-scale input peak was 0.999229, so this capture does not assert bit-exact output.
- Before repair, sustained 0.75 input reached 1.051568 before limiting at middle depth. After repair, measured peaks at depth 0/0.5/1 were 0.338023/0.341416/0.323388.
- Impulse early-tail occupancy increased from 2.3–11.0% to 22.9–71.3%. Tail decay, finite values, dry bypass, rapid controls, and subtle stereo tests remain separate from listening acceptance.

Probe reports: `/tmp/musicos-reverb-baseline.json`, `/tmp/musicos-reverb-repaired.json`, `/tmp/musicos-reverb-final/report.json`. Test commands and build identity are recorded below after final verification.

## Acceptance limitations

Human listening for distortion, pumping, room character, and automatic DJ musical quality remains required. Software WebGL checks do not prove native 120 Hz rendering or macOS edge-bounce alignment. Windows/Linux are unverified. Sparklist model/listening acceptance and macOS system-control acceptance remain separate. No model assets were published.

## Automated checks so far

- UI TypeScript/Vite build and 67 frontend unit tests passed.
- 40 database tests passed; expanded relocation test also verifies favorites, playlist membership, overrides, restoration, unmatched paths, and unchanged bytes.
- 76 desktop-core tests passed, plus stable ignored-fingerprint and worker-rescan tests.
- 105 Tauri tests passed serially, followed by the additional transactional ignore/restore test. The optional 281 MB install checksum fixture required a realistic 60-second debug-test deadline; production verification/checksums are unchanged.
- Compact/wide creation, supplied square covers, preview play/pause, theme persistence, 40 management size/scale/palette combinations passed. Theme fixture rendering was corrected to update its background layer as well as inherited tokens before screenshot review.
- Final browser rerun: pointer placement/clamping, manual-only picker, Escape, ignored persistence/restoration, multi-song selection, and compact integrations passed. Eight depth checks passed, including retained canvas identity, 1,000-album rapid reversals, bounded RAM/GPU counters, context recovery, one-time reveal, reduced motion, and expanded-image geometry.
- Earlier parallel browser runs collided over a shared preview server/output directory; reruns are serialized. A cold software-WebGL startup under concurrent release compilation exceeded the 15-second readiness gate; the unchanged test passed after compilation ended. These failures are retained in `/tmp/musicos-library-*.log` and are not hardware performance evidence.

Final backend totals: **77 desktop-core**, **40 database**, **106 Tauri** tests passed; frontend **67 tests** passed. The serialized browser run passed **15 tests**, including the 40-case management matrix. Final screenshot review follows a corrected theme-fixture rerun.

The embedded sidecar and ONNX runtime remain unchanged:

- `musicos-mac-aarch64-apple-darwin`: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- `libonnxruntime.1.31.0.dylib`: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

Final visual/interaction reruns passed: **9 queue/management checks**, **6 creation/everyday checks**, and all four viewport-corner menu clamps with keyboard focus restoration. Creation was checked in 40 size/scale/palette combinations, with square artwork and bounded dialog geometry. Reviewed compact dark and adaptive creation screenshots, the dark metadata comparison/list surfaces, and the supplied integration glyphs separately from geometry assertions.

The managed-source protection is enforced in storage as well as the UI. The collection persistence fixture now uses a removable referenced folder rather than trying to disable managed storage; the subsequent full **106-test app suite passed**.

GPU recovery deliberately bounds consecutive automatic reconstruction attempts; ordinary images remain the fallback if the graphics context cannot stabilize. Navigation or explicitly toggling depth rebuilds the view. Native GPU, bounce, performance, and listening acceptance remain below until checked on the delivered package.

## Delivered package identity

- Package: `target/release/bundle/macos/MusicOS.app`, version 0.1.0, Apple Silicon, 98.30 MiB.
- Executable SHA-256: `7fc24c1b1afecddef8035c6c9aedc7844585a771fc00fecf33c4823d70e13474`.
- Ad-hoc codesign applied; `codesign --verify --deep --strict` passed. This is a local signed development package, not a notarized public release.
- Base Git revision: `27e215f2dd751dcc8f1172717ff66027da2a12d3`; includes the existing uncommitted work and this repair pass.
- UI build passed. Embedded sidecar and ONNX hashes match the preserved baseline above.
- Final package log: `/tmp/musicos-library-package-complete.log`; backend `/tmp/musicos-library-rust-delivery.log`; UI interactions `/tmp/musicos-library-ui-delivery.log`.

## Package acceptance checklist

| Check | Result |
| --- | --- |
| Build, signature, runtime preservation | PASS |
| Reverb production-mixer capture and finite/headroom/tail tests | PASS (automated) |
| Requested/applied listening and queue regression tests | PASS (automated) |
| Depth RAM/GPU budget, reversals, resize, context fallback | PASS (software WebGL fixture) |
| Creation/theme geometry and screenshot review | PASS (browser fixtures) |
| Ignore/Restore, rescan exclusion, stale guard, transactional persistence | PASS (automated) |
| Folder soft removal, restore, verified relocation and protected managed source | PASS (automated) |
| New package loaded into the running app | PASS: loaded after authorized pause; later playback state verified paused |
| Native queue/Audio/depth checks on this package | PASS: separate Up next and Audio, requested/applied reverb acknowledgement, two-page depth scroll/reversal; edge bounce/context loss stress remains open |
| Human reverb/DJ listening acceptance | OPEN |
| Actual 120 Hz dropped-frame/rendering acceptance | OPEN; older telemetry is not attributed to this build |
| macOS system controls and Sparklist model listening acceptance | OPEN, separate gates |
| Windows/Linux | UNVERIFIED |

## Follow-up: flat Settings and reverb direct gain (October 15)

The feedback identified a real mix problem. The old reverb formula reduced the direct recording by roughly 3.06 dB at 45% room and 7.47 dB at 100%. The revised processor keeps unity direct gain and adds the damped room return. It does not increase feedback to compensate. Existing smoothing, finite parameter guards, wet-only stereo decorrelation, preallocated diffusion, pause freeze, seek clearing and safety limiting remain. The pad now says Room amount rather than dry/wet.

Production-mixer captures, 8-second excerpts at 48 kHz stereo:

| Source | Dry RMS | 45% room RMS | Change | 100% room peak | Tail energy |
| --- | ---: | ---: | ---: | ---: | ---: |
| Moon Crystal (FLAC) | .324562 | .326340 | +.0474 dB | .997003 | 10.63888 |
| I THINK (FLAC) | .304171 | .304663 | +.0140 dB | .991281 | 2.591326 |

Impulse and sustained-input depth 0/.5/1 probes remain finite, preserve direct impulse amplitude, decay, and stay below .782 pre-limiter with a sustained .75 input. The new direct-signal regression verifies unchanged early samples at .25/.45/1 amounts. These are automated measurements, not human listening acceptance. Captures: `/tmp/musicos-room-send-output`, `/tmp/musicos-room-send-igor`; stability: `/tmp/musicos-room-send-stability.json`.

Settings now use compact solid groups and rounded flat controls. Playback implementation information moved into Details. Integration switches have no separator above them; the two integration rows share one group. Source actions use the same rounded controls. Collection editor artwork opens separately and preserves the current preview. The manual-playlist picker includes square artwork, names and counts. Sparklist dialog/editor boxes are solid themed surfaces; the supplied white Shuffle/Play treatment and cover artwork are retained. Chromatic cover colors now outrank gray collage areas when sufficiently prominent.

Validation: UI build; 68 frontend tests; 78 desktop-core tests; 8 browser tests including 40 creation size/scale/palette cases, menu clamps, ignored persistence, separate song/artwork selection and editing. An ignored-tab display race and test reload-before-write race were corrected. Browser screenshots were visually inspected separately. Synthetic palette screenshots must update root tokens too: changing only shell tokens incorrectly leaves inherited derived surfaces in the blue palette.

Prior package native evidence: queue and quick Audio remain independent, correct FLAC specifications, reverb delivery acknowledged while paused and restored off, depth covers survive two-page scroll/reversal. A few short compositor traces showed median 8 ms and p95 9–14 ms; this is not full 120 Hz hardware acceptance. All listening and cross-platform gates remain open.

Follow-up package identity and native smoke results are recorded below after signing/loading.

### Follow-up delivered identity

- Signed development bundle: `target/release/bundle/macos/MusicOS.app`, 98.30 MiB.
- Executable SHA-256: `e3e8bb7e9f4984f45ad88c8b29b55b06249680fffc3a12838e67d51973aa9962`.
- Ad-hoc deep/strict signature verification passed. Embedded sidecar and ONNX hashes remain unchanged.
- Loaded at 21:37:36 EDT, PID 83217, exact bundle executable path verified. Playback was paused before quitting. Restored paused position to 8.1 seconds after launch; 477 songs, 11 favorites and saved collections present (library had changed through user indexing since the earlier 443-song check).
- Native Integrations screenshot inspected: flat solid presentation, supplied icons, no lines above switches, Discord retained Connected/paused, Last.fm retained Not connected. Credentials unchanged.
- Quick Audio retained user speed .62×, linked pitch −8.3 st, eight-band flat EQ, volume 100%, depth .5. Setting reverb 45% while paused changed Applying to acknowledged Applies to all songs. Mac locked before restoration-off/closing could be confirmed; manual unlock requested. Do not count this as human listening acceptance.
- Browser palette test initially omitted root tokens and displayed false blue-on-dark contrast; corrected fixture updates both shell and root. The adaptive fixture now supplies complete preferences, avoiding a test-only NaN interface scale. Dark and adaptive screenshots inspected after correction.
- Logs: `/tmp/musicos-friendly-package.log`, `/tmp/musicos-friendly-final-build.log`, `/tmp/musicos-friendly-final-ui.log`, `/tmp/musicos-friendly-final-browser.log`, `/tmp/musicos-friendly-theme-browser.log`, `/tmp/musicos-friendly-core.log`.

Follow-up cleanup confirmed on the later organizer build: reverb restored off, applied state acknowledged, room 0%, depth .5; user speed/pitch and remaining listening settings preserved.
