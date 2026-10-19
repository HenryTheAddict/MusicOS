# Reverb and automatic DJ recovery — 14 October 2026

## Findings and repair

The reported completely dry effect was not reproduced in an isolated capture of the production mixer: a real recording produced a nonzero wet signal and a decaying tail. This is evidence of working DSP, not proof that every UI request reached the user's device. Application state now distinguishes requested settings, mixer-applied revision, applied reverb amount, and the number of frames modified by reverb before master volume. Unconfirmed settings expose Retry inside Audio. Retrying the identical saved preference resends it when playback is mismatched or unconfirmed; a preference-file save alone does not establish application.

The mixer previously returned silence immediately when decoded source frames ended, bypassing reverb-tail processing. It now renders the room tail after EOF and waits for it to decay before automatically pausing the last song. Manual navigation/seek clears old tails; pause freezes them. Rate acknowledgement is tracked separately from effect revisions, so reverb/EQ changes do not depend on decoder progress, particularly while paused. Reverb defaults off and saved amounts remain unchanged. The existing processor was retained rather than increasing gain without evidence.

Incoming decoder failures could leave a prepared entry present indefinitely. Generation-specific decoder failure reporting and five-second readiness deadlines now terminate that state. Failed decoding skips the affected queue entry for the session; preparation starvation/deadline retries are bounded to two attempts. Entries are not removed, and explicitly selecting a repaired entry retries it. All-failed queues and repeat modes terminate safely instead of retrying a broken file forever. Stale decoder jobs are discarded before opening files.

Requests have stable identities and requested/preparing/accepted/active/promoted outcomes, plus rejected/canceled/failed results. Decoder diagnostics remain bounded and separate from concise playback messages. The coordinator does not treat send as acceptance, retries missing outcomes, and leaves preparation deadlines to playback. Accepted schedules and preparations are protected from routine replacement and energy rearrangement. Each promotion gives the planner a short preparation window; DJ tempo correction eases back to the user's baseline after promotion.

A cancellation epoch invalidates old schedules even when the command ring is saturated. New schedules in the current epoch remain valid. Navigation reserves the complete command sequence before changing the authoritative current entry. Existing sample-clock scheduling, confidence thresholds, user/DJ tempo separation, manual pins, and accepted-only rail marks remain in place.

## Verification

- 72 desktop tests passed, including output-level dry/wet capture, post-EOF tails, pause freeze, applied reverb revisions, native automatic overlaps, corrupt incoming recovery, unchanged duplicate-capable queue identities, bounded preparation expiry, paused deadlines, protected accepted schedules, and saturated command cancellation/navigation.
- 96 Tauri tests passed; all examples compiled before the final internal cancellation hardening, and the final package compiled those updated contracts successfully.
- 63 frontend tests passed, including unconfirmed-application error/retry and clearing the error after mixer acknowledgement.
- Five Audio browser scenarios passed, including the 40-case window/scale/palette matrix. Existing layout and supplied controls were preserved.
- The final native no-seek probe played EVERYTHING MUST GO → GL (early take) → Sometimes… from their beginnings in a disposable library, using real decoder workers and the device-output mixer at zero master volume. Two ending-blend activations and deck promotions were observed with complete request lifecycles. It did not edit the user's queue or music. These short, uncertain recordings verify automatic fallback operation; they do not establish beat-matched musical quality.
- A real FLAC excerpt was captured through the production mixer: 384,000 source frames (eight seconds), wet-minus-dry RMS 0.04193 and post-source wet-tail energy 1.16036 at 45%. Dry/45%/100% WAVs and machine-readable reports are in `out/audio-repair/`. Samples are finite and limited below full scale. The source peak exceeded the existing 0.9 safety-limiter threshold, so the real-recording dry render is not bit-identical; the below-threshold dry-bypass regression is exact.

## Acceptance limits

No subjective listening pass is claimed. The device-output transition probe was muted; the wet render is inspectable audio, not an OS loopback recording. Packaged audible reverb character, confidence-supported beat matching, late/stressed real-recording playback, and broad codec listening acceptance still need human listening. Windows/Linux remain unverified. Source files, model assets, collections, canonical artwork, and embedded runtime were preserved.

Final signed build identity and load status follow below.

## Final package

Executable SHA-256: `b00108f2d9f3e8e50eb27f0b65e7d30d9138287f2688b55ee32bb3096da0c3d3`.

Package: `target/release/bundle/macos/MusicOS.app`. Deep ad-hoc signing and strict verification passed. The final no-seek native run observed two automatic overlaps and promotions in 58.13 seconds. Its diagnostic report is `out/audio-repair/automatic-native.json`.

Embedded sidecar source SHA-256 remained `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`; ONNX runtime source SHA-256 remained `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

Playback was confirmed paused on Solitude at 77 seconds before the final restart. The earlier repair build had already restored the library and playback without starting music; the user then used the app normally while final queue-saturation hardening was tested.

## Loaded-build verification

The final executable was modified at 16:31:23 EDT and its process started at 16:32:38 EDT from the intended package. The app restored Solitude at 77 seconds, paused, with 443 songs, 10 favorites, and the existing collections. In the packaged native Audio panel, enabling Reverb selected 45%; the preference file saved 0.45 and the panel advanced from Applying audio controls to Applies to all songs only after the mixer acknowledgement. Reverb was then restored to its original off/0 preference, saved, and acknowledged. Playback remained paused at 77 seconds, and the Audio panel was closed. No audible device-listening verdict is implied by this control-path verification.
