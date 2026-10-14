# Artwork, controls, and Sparklist setup — 2026-10-10

## Implemented

- One persisted canonical album cover is projected into library tracks, queue/Now Playing presentation, adaptive colors, and macOS Now Playing artwork. File tags and embedded images are not rewritten. Missing art uses the shared placeholder until a valid cover is found.
- One themed switch component replaces conflicting Settings controls. Song duration and Now Playing time remain separately persisted. Shared action alignment, artist-strip gutters/two-line titles, rail/time grid spacing, transparent corner levels, and palette-derived controls are corrected.
- Row Play morphs into three rounded level contours over 180 ms, reverses from its interrupted progress, and targets the current telemetry heights. Transport Play/Pause is separate. Visual bass/mid/treble calibration is fixed per band with bounded compression and attack/release smoothing, outside the audio callback; audible EQ is unchanged.
- Cover hover uses a finite stretch/settle; visible grid artwork has bounded ±6 px parallax through one animation-frame loop per grid. Reduced motion disables these effects.
- The supplied Sketch Filter icon opens a top-layer popover with persisted sorting, genre, quality, favorites, availability, and optional Singles & EPs sections. Classification uses explicit tags or verified release metadata, never track-count guesses. Unknown releases remain Albums.
- Song title/artwork pointer dragging suppresses native selection and image dragging, uses capture and a movement threshold, and inserts independent queue entries at the indicated position. Internal dragging is isolated from file import.
- Sparklist creation reports consent/model/installation/analysis readiness, links directly to model setup, prioritizes seed analysis, and refreshes without restart. The development setup accepts only the pinned, checksum-verified staged replacement candidate. Consent remains off by default; no assets are published.
- A desktop decoder iterator size-hint overflow found by real MP3 transition testing is avoided by explicit bounded reads. The embedded runtime is unchanged.

## Evidence

- 40 frontend unit tests passed.
- 67 Tauri library tests passed, including migrations, canonical cover persistence, preference validation, model installation failures/checksums/cancellation, and embedding invalidation.
- 52 desktop core tests passed, including independent queue insertion identities and visual band calibration.
- 16 browser interaction/layout tests passed. A subsequent four-test layout run verifies artist Play transparency and Settings switch geometry; five motion tests verify the final morph. Coverage includes 720×480 and 1280×800, 80/100/125/150% scales, reduced motion, and palette-derived surfaces. These are geometry/behavior checks, not a claim of visual approval.
- Screenshots: `out/rebuild-qa/consistency-settings.png` and `out/rebuild-qa/consistency-artist-strip.png`.
- Native report: `out/rebuild-qa/consistency-native-audio-beta-report.json`. Uses real local recordings, an isolated temporary database/model installation, bundled Apple Silicon ONNX Runtime, and muted native CoreAudio output. User music, preferences, and playback queue are not changed by the probe.
- The native probe generated eight real embeddings in 25.94 seconds, then exercised cached command generation, Choose For Me, deterministic seed-plus-mood ranking, explicit feedback, saving, refresh, manual conversion, and restart persistence. It also observed scheduled/active transition states, incoming deck promotion, and pause source-time freeze.

## Remaining acceptance gates

- Audible DJ listening, pitch/limiting assessment, and blind Sparklist relevance evaluation remain pending. An activated native transition does not establish listening quality.
- macOS system UI remote-command acceptance and packaged-app drag/autoscroll coverage remain to be verified manually.
- The actual user's beta consent is not silently enabled for testing. Native cached-embedding service checks do not replace packaged creation/setup acceptance or user visual approval.
- Windows/Linux are unverified. Public model hosting/publication and production model promotion remain gated.

Packaged build identity and loaded state are recorded separately in `out/rebuild-qa/feedback-build-identity.txt`.

## Follow-up: floating drag and morph interruption

Floating Audio previously dismissed on pointer-down outside the panel, including the start of a song drag into it. Dismissal now waits for a completed click; drag clicks are suppressed after the movement threshold. Escape also suppresses the synthesized click on physical pointer release, keeping the panel open after cancellation. Regression coverage verifies successful floating-panel insertion, insertion before a specific row, duplicate entries, cancellation, autoscroll, and ordinary outside-click dismissal.

The morph now draws its initial contour synchronously to avoid a blank first frame, and reduced-motion endpoints reset interruption progress. A new deterministic animation test verifies that cancellation removes earlier frames and reversal resumes from the current contour. 41 frontend unit tests passed; eight Audio/queue browser tests and nine queue/motion browser tests passed. Native inspection remains blocked by the locked Mac.


## Packaged native inspection after unlock — 2026-10-11

Signed build `2ecef4a615b9fea1631cafe1381772cb3f364ad663c6b2265921e3cca48f5fd2` is loaded in the intended app. Playback restored paused at The Wizard, 102.9 seconds. Junk artwork matched its album card, header and Audio queue. Native title dragging into floating Audio successfully inserted a temporary entry without activating file import; removing that entry restored all seven original upcoming songs and their order. Playback remained paused at the original source position.

Native Appearance shows distinct, aligned Song times switches. Audio shows the existing user configuration and beta consent off. Artist-strip covers/titles, transparent Play, adaptive surfaces, and the Sparklist consent/setup state were inspected in the packaged app. The user resumed interacting with the Sparklist modal, so inspection stopped without dismissing it or changing consent. This supersedes the earlier locked-Mac launch blocker.

Packaged long-queue autoscroll/cancellation, audible DJ evaluation, macOS system remote-command acceptance, and model listening evaluation remain outstanding. The native cached-embedding probe validates service behavior, not a completed user-consented packaged setup/generation session. Windows/Linux remain unverified.

## Motion, portrait framing, and consented beta follow-up — 2026-10-11

The latest hover response is a finite 240 ms stretch, compression, and settle. Reversal captures the displayed scale instead of resetting it. Artwork parallax now reaches ±14 px inside fixed overscanned frames, with timestamp-based smoothing and batched geometry reads. SVG morph frames no longer trigger document-wide rounded-surface rescans. Reduced motion disables elastic movement and parallax. Portraits use local Apple Vision face rectangles, conservative confidence filtering, headroom, and separate cached crops; original images and tags remain untouched. A real cached portrait produced a centered 640×640 crop through the native adapter.

Canonical artwork now validates decoded image contents, retaining the prior valid cover when a replacement is corrupt. Album grouping normalizes typographic apostrophes while retaining explicit editions and verified distinct releases. Settings exposes bounded metadata-cache recovery with concise status. Sparklist Save now validates and persists the exact displayed preview instead of regenerating it while analysis coverage changes.

Verification: 44 frontend unit tests, 73 Tauri library tests, and 52 desktop-core tests passed. Eighteen browser interaction/layout checks passed, followed by five consistency checks including actual scroll-driven parallax and reduced motion. Comparable fixture screenshots are under `out/rebuild-qa/polish-matrix`: both requested viewport sizes, four interface scales, and blue/light/dark/vivid palettes, plus artist-strip adaptive transparency. Inspection of small/large captures found aligned controls and a transparent strip; these checks do not constitute user visual approval.

With explicit user consent, beta was enabled in the intended packaged app, the exact checksum-verified staged replacement model installed, and 442 of 443 local songs analyzed. Seed-plus-Dreamy preview, regeneration, Less like this, saving a 25-song Sparklist, and conversion to a manual playlist worked. The saved manual sequence matched the Sparklist and retained feedback. Test collections are named “Sparklist beta test” and “Sparklist beta test — playlist”. Model assets were not published. The replacement remains experimental pending blind listening evaluation.

The user subsequently started Aerodynamic. The final build is staged separately to avoid interrupting playback. Its signed executable SHA-256 is `7dfd45e17e0429df95298b9d3cc0f00f6c54496d75e22d7012663d97c464f916`, at `out/rebuild-qa/polish-delivery/MusicOS.app`. It has not yet replaced the running process. The previously loaded executable was `448c63b20bd511e42153c60329fac9e8d5880e50fc1b4f6c396a31158e00bd6e`.

Outstanding gates: load/restart validation of the final staged build, packaged long-queue autoscroll/Escape, audible DJ quality, macOS system remote commands, and blind model relevance. System Control Center inspection timed out in UI automation; adapter tests do not replace that acceptance. Windows/Linux remain unverified. Embedded runtime remains unchanged by this follow-up.

## Filter, row-state, collection-color, and analysis feedback — 2026-10-11

The Filter popover now uses compact themed dropdowns with custom chevrons, consistent rounded controls, a separate compact toggle section, and an aligned rounded Reset button. Browser checks confirm persistence and viewport containment at 720×480; settled captures are `out/rebuild-qa/filter-menu-polished.png` and `filter-menu-small.png`.

Mouse focus and selection no longer independently expand a previous song row: reveal follows pointer hover, visible keyboard focus, or the actual current playback indicator. Play/EQ is explicitly centered in its hit area; levels rise from a common baseline. Keyboard reveal, reduced motion, row number preservation, and all requested interface scales passed the six existing motion/song-action browser checks. Forty-four UI unit tests passed.

Personal playlist controls derive their tint from their displayed cover. Sparklist Play/Shuffle controls use a rainbow, moving on hover/focus, plus four finite particles. The original shaded CollectionShuffle SVG canvas is omitted while retaining its supplied glyph; CSS owns the surface. Reduced motion disables moving gradients and particles. The collection-color browser check passed for sidebar and detail controls, and confirms regular playlists do not receive a rainbow. Screenshot: `out/rebuild-qa/collection-control-colors.png`.

Read-only inspection found 442 persisted structural analyses, but zero met the 0.5 beat-confidence gate (median 0.0042; maximum 0.2512). That explains the frequent two-second unwarped ending fallback. The original analyzer rounded the beat period to an integer hop, accumulating phase drift. The replacement preserves fractional periods, refines within ±3%, and requires matching onset phase evidence in both halves. Silence, flat envelopes, isolated transients, and fractional-period multi-minute patterns pass. Cache version increases to dsp-3/structure-2; embeddings stay intact and structural analysis must be refreshed. The Settings analyzed count now reads persisted cache coverage even while a batch runs, rather than showing just that batch. Planner diagnostic records include reasons and confidence outside the audio callback.

A read-only real-recording probe (`src-tauri/examples/structure-qa.rs`, `/tmp/musicos-structure-qa.log`) found higher but still insufficient confidence for Do It, Try It (0.0099→0.1037) and Solitude (0.0024→0.0448); Moon Crystal remains uncertain. These tracks must continue using fallback. This correction does NOT establish acceptable musical beat/phrase analysis or audible DJ quality. A validated beat/phrase analyzer and annotated listening acceptance remain open; confidence thresholds were not lowered to manufacture successful transitions. Seventy-five Tauri library tests passed before the final unclipped scoring refinement; the four structural tests passed after it. No active playback was interrupted.

Latest signed staged executable supersedes the earlier hash: `1f3a6fd92433e26f63a12c691902de9c50ca91cfd8a4a9d283169f26c8ff1b21`. Signature verified deep/strict. It remains staged separately, pending the existing restart choice while user playback is active.

## DJ beat tracking and album duration follow-up — 2026-10-11

Full-track structural analysis now uses log-spectral onset changes, local loudness removal, tempo autocorrelation, and an original Rust dynamic-programming beat path. It no longer treats an excerpt's weak tempo score as a hard ceiling on full-track beat confidence. Method reference: [Ellis, Beat Tracking by Dynamic Programming](https://www.ee.columbia.edu/~dpwe/pubs/Ellis07-beattrack.pdf). Phase candidates require several measured structural changes; a regular beat train alone does not assert a phrase phase. Repetition checks compare duration and changing energy/timbre profiles rather than average instrumentation. Estimates remain estimates; silence, random noise, missing attacks, and fractional beat periods have regression coverage.

A read-only optimized native scan covered all 42 local tracks across Discovery, Currents, and Junk: 36 passed the unchanged 0.5 beat-confidence gate, seven supported recurring boundary candidates, and 20 of 1,722 ordered pairs yielded supported musical plans (remaining pairs use fallback). Total structural scan time was 12.26 seconds on this host. This is functional coverage, not annotated beat accuracy or listening acceptance. Tempo can still resolve at half/double time. Detailed local probe output: `/tmp/musicos-dj-three-albums.log` and `/tmp/musicos-dj-plan-report.json`.

The 20 Hz DJ coordinator now reads lightweight progress and per-track revisions instead of cloning all full-track analysis arrays repeatedly. Unrelated completed analyses no longer restart the current pair's transition preparation or rearrange a queue. Planner diagnostic reasons distinguish unavailable structure, uncertain beats, tempo incompatibility, and absent supported boundaries. New structural cache version is dsp-4/structure-3; embeddings remain unchanged, and structural results require refresh.

Album duration cells now have an explicit column on the same row as title and actions, instead of being hidden while their grid placement leaks into another row. About this album lives inside the header below contributor portraits, wraps with multiple contributors, and expands in place. At narrow content widths it takes a full-width row. Two layout checks covering 720×480 and 1280×800 at 80/100/125/150% passed, including 18 contributors. Screenshots: `out/rebuild-qa/compact-contributors-720.png`, `compact-contributors-1280.png`, and `album-duration-*.png`. Forty-four UI unit tests, 76 Tauri library tests, seven song/motion browser checks, and 52 desktop-core tests passed. Audible acceptance and final loaded build identity are recorded after the delivery check below.

## User-directed visual reversions and wide-screen fixes — 2026-10-11

Latest user direction supersedes the cover-color/rainbow experiment and inline About placement above: restored the supplied soft-gradient Shuffle Play artwork, removed collection particles and rainbow surfaces, and returned About this album below the header. Duration-column and native audio fixes remain. No additional visual redesign was added.

Wide-grid captions now participate in row sizing (`display:block` inside a column flex control), preventing two-line titles and credits from colliding with the next artwork row. Added an immediately persisted Appearance > Artwork parallax switch. Disabled parallax removes both displacement and overscan. Virtualized card mounting now updates observation incrementally, preserving the offsets of existing visible artwork rather than restarting all of them. Artist popovers are hidden during initial measurement/positioning and only enable cursor-follow transitions afterward, preventing travel from the screen corner. Local face framing is loaded in the native app; this session found 29 cached face crops and seven honest no-face fallback records.

Native isolated DJ verification passed with real Aerodynamic → Crescendolls audio: accepted 3.90-second four-beat plan, scheduled readiness, automatic activation, incoming deck promotion, source-position accounting, paused source-clock freeze, and seek cancellation. Output was muted and the user's queue was unchanged. Record: `/tmp/musicos-dj-transition-native-final.log`. Audible listening remains unverified.

Final restored-control build executable SHA-256: `fbd65c133f639cdeb629d5609e8ef36c1fa675ab72842a19c800d98ed6ed40d0`. Ad-hoc deep signature verified strict. Installed while playback was paused at Walkway Blues 33.7 seconds. Native inspection follows below. Structural cache refresh is still needed to apply the new beat tracker to existing cached analyses; embeddings remain intact.

Restored-control build was opened in the intended app and inspected: gradient Shuffle Play canvases are restored, wide-grid captions occupy their own rows, playback resumed from the paused 33.7-second source position. The user subsequently resumed music. New DSP-4 results are already being cached for upcoming DJ tracks (eight observed during the check), without rewriting embedded tags or embeddings. The latest explicit motion preference changes hover stretch to one 480 ms soft bounce with a 260 ms exit, reduces amplitude from 6.5% to 2.5%, and softens artist-popover entrance. That small follow-up is staged separately until a paused restart; it does not alter transport or song reveal timings. The two final wide-grid/gradient browser checks passed after awaiting the asynchronous preference write, and the parallax preference's native atomic restart persistence test passed.

Softer-motion follow-up is now built and signed: staged SHA-256 `90f904ec2a199ca10a3f8149cc9a0409239c83d83ea991e25b9818b9d82faf9f`. Forty-four frontend tests and two focused reveal/popover browser checks passed. It remains staged because playback was active at the final check (Walkway Blues 268.8 seconds); the loaded reverted-controls build remains `fbd65c…`. No second restart interrupted music.

User-requested restart completed: softer-hover executable `90f904ec2a199ca10a3f8149cc9a0409239c83d83ea991e25b9818b9d82faf9f` is loaded in the intended app. Native UI restored Lost in Yesterday at 5.2 seconds, paused, retaining favorites and saved collections.

## Softer motion, real filter menus, and resize stability — 2026-10-11

Hover stretch now lasts 820 ms, with a smaller 1.8% peak and a 460 ms smooth exit. Per-segment easing is applied once, avoiding the previously compounded acceleration. Artist-name entrance shares the slower duration; song-row reveal and Play-to-EQ retain their separate timing. Transport Play/Pause uses matched quadratic-rounded contours over 360 ms, reverses from the visible progress, and returns to the supplied endpoint SVG. Both fills are painted over both outlines to avoid a hard seam while the shapes overlap. Reduced motion changes state immediately.

Filter fields now open themed Radix radio-option menus, with selected checks, typeahead, arrow-key navigation, bounded scrolling, collision handling, and focus return. Content remains within the native popover top layer. Visual review caught and fixed a stacking issue with positioned switches behind the option menu. Escape closes the option menu first, then Filter. Existing filter persistence is retained.

Virtual grid keys no longer include the continuously changing row height. Images and card state remain mounted during live resizing; columns are recalculated after 140 ms of idle, with one 280 ms rearrangement instead of restarting transforms on every width change. Removed competing image scale animations during rearrangement. Row estimates now match the actual caption/gutter height instead of overshooting by 32 scaled pixels.

Verification: 45 frontend unit tests passed, including interrupted rounded transport morphs and visible-state hover reversal. Browser checks cover filter persistence, selected/keyboard menus at 720×480 and 1280×800, menu stacking/hit testing, rapid transport reversals and reduced motion, stable virtualized card identities during continuous resizing, settled layouts from 720 to 2560 pixels, long captions at four interface sizes, and the unchanged supplied Shuffle Play gradient. Captures: `out/rebuild-qa/filter-options-720.png`, `filter-options-1280.png`, and `grid-resize-settled.png`. No wallpaper feature was implemented; the Pixel wallpaper request was a question only.

Native delivery identity and load verification follow after packaging. Existing audible DJ, blind Sparklist, system media-command, and other native acceptance gates above remain open; Windows/Linux remain unverified.

Loaded packaged follow-up SHA-256: `8614766d75ddea7b6674898becb0fed86731fb363d03c1fa5f81299f777a6172`. Signature verified deep/strict. Restart occurred while paused; native UI retained Lost in Yesterday at 5.2 seconds, 443 songs, six favorites and saved collections. The actual native Sort menu rendered with the adaptive palette and selected check. Escape closed its options and restored Sort focus; a second Escape closed Filter and restored its trigger. Native playback was left paused and filter preferences were not changed.
