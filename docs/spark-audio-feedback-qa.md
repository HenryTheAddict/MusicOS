# Sparklist and connected EQ feedback — 13 October 2026

## Delivered

- Removed the separate frequency-response graph. A smooth curve now connects the EQ slider handles directly, using the same handle centers and gain range. Fixed-width output slots and grid columns prevent number changes from resizing Audio.
- Reverb no longer applies its amount twice. Dampened stereo feedback and a single wet mix provide a stronger effect at modest amounts, with smoothing, headroom, exact dry output at zero, bounded buffers and tail decay. Enabling it initially selects 45%; existing stored values are preserved.
- Ordinary non-depth grid parallax returns from 1.04 scale to 1.0 on hover/focus and eases back on leave. Song-row thumbnails do not receive this grid effect.
- Shuffle/Play uses the supplied SVG's white fading surface. The green stop is replaced with cover-derived surface colors; Sparklists use a cover-keyed multicolor treatment. Removed competing button backgrounds that produced a second shape beside the SVG. Reduced motion stops gradient movement.
- Sparklist creation uses optional seed search rather than an always-visible song search/list, removes the full ready-state embedding count, and puts incomplete coverage in a disclosure. Setup requirements remain explicit when needed.
- Automatic cover choices select deterministically from the nine supplied Sketch Sparklist covers using the existing audio-derived parameters. Existing manual and generated cover choices stay intact. Regenerate artwork changes only the cover.
- Cover-picker choices remain square. Sparklist buttons have readable paired colors and no inset embossing; primary Name & save/Save actions fill their available width.
- Names start empty, preserve typed edits, and offer only an optional cleaned seed-title-esque suggestion. Removed generic mood/color slogans.
- Preview listening awaits the authoritative playback command, surfaces failures inside creation, and supports pause/resume without silently restarting the sequence. Preview generation and naming never start playback automatically.

## Verification

- UI production build passed; 62 frontend unit tests passed, including curve geometry, naming, setup readiness, explicit preview listening and retryable preview failures.
- 60 desktop native tests passed. Six focused listening DSP checks subsequently passed, including the new modest-amount wet-energy test, dry bypass, damping/decay, bounds and EQ response.
- Eleven unique browser scenarios passed across collection creation and Quick Audio: square covers, explicit preview listen/pause/save, name preservation, fixed panel dimensions at extreme values, hover return to full artwork scale, setup gates, persistence/reset, thumb-free volume, Settings adaptive contrast, and window/scale checks. The Audio matrix includes 40 combinations of two windows, four scales and five palettes.
- Rendered connected handles, square cover selection, creation stages and cover-based controls were inspected. Browser Sparklist playback uses an explicitly requested development-only fixture; this does not constitute real-model/native listening acceptance.
- No source music, model weights, saved collections or embedded runtime were changed by this pass.

## Signed package and load

Executable SHA-256: `7c45efb550883c4c833575ec7e3ab074e69fb8339df9c23b95c34403ca687698`.

Package: `target/release/bundle/macos/MusicOS.app`. Deep ad-hoc signing and strict verification passed. Executable modified at 22:28:19 EDT; new process started at 22:29:18 EDT. Playback was confirmed paused before restart. The same track, position (The Game of Love, 104.9 seconds), 443 songs, 10 favorites and saved collections were present after launch. A subsequent interaction guard reported the user changing the app, so further native UI automation stopped.

Audible reverb character, native Sparklist preview listening, musical DJ acceptance and model recommendation quality still require listening checks. Windows/Linux remain unverified. No claim of bit-perfect processed playback or measured 120 Hz presentation is made.
