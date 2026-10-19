# Native depth resize and scrolling repair — 2026-10-14

## Reproduced failure

In the packaged macOS app, native window zoom left old depth-rendered covers floating over the newly laid-out album grid. The stale covers persisted after resizing settled. The underlying adaptive palette remained consistent during this reproduction; the previous theme-background change did not address this artwork failure.

## Repair

- Resize immediately hides stale canvas output and returns ordinary artwork.
- Observe individual artwork bounds and settled grid-layout changes, including visual-viewport resize.
- Re-measure after layout animation finishes. Publish depth-ready state only after a successful fresh render; GPU errors restore ordinary images.
- Bound canvas backing memory to eight million pixels, retaining up to 2× device scale where space permits.
- Remove synthetic wheel-driven scroll translation, stretch, and artwork blur. Scrolling no longer deforms the viewport or its fixed depth compositor. Cover pointer depth and hover motion remain independent.

## Automated verification

- Production UI build passed; 63 frontend tests passed.
- All ten depth/motion/resize browser cases passed.
- Seven depth cases passed again after the final measurement cleanup.
- Includes wide/compact resize while scrolled near the end, artwork geometry alignment, virtualized-grid reflow, navigation, reduced motion, pointer depth, one-time reveal, and GPU-loss restoration.
- Native resize acceptance is recorded below after testing the packaged repair. Source music, collections, listening preferences, and the embedded runtime are preserved. Windows/Linux remain unverified.

The final version also recreates the WebGL surface after 160 ms without further native/visual-viewport resize events. During resizing it shows ordinary artwork. Cached decoded images and depth maps are reused; inference is not repeated. Old context-loss listeners are detached before teardown so they cannot alter readiness on the replacement surface. All seven depth cases passed again, including an assertion that the old canvas is removed after each resize.

The grid now reads its width on native-window/visual-viewport resize as well as ResizeObserver delivery. A dedicated regression suppresses grid ResizeObserver callbacks and verifies columns expand and return correctly. The depth surface also detects viewport size changes through its observer as a fallback. Software WebGL cold starts required a 15-second readiness/geometry allowance; strict 1.2-pixel alignment checks remain unchanged. The final four-size depth/canvas-replacement case passed (39.5 seconds total across all sizes). The other ten depth/motion/fallback cases passed; earlier 5-second cold-start/geometry timeouts are retained in logs and are not hardware performance results.

Packaged executable SHA-256: `86e05abcf493bd4813daa80500a22be6ff6c9561882db1901bcbc9488324fc8a`. Deep ad-hoc signing and strict verification passed. Embedded sidecar/ONNX hashes are unchanged. Playback was confirmed paused before reloading.

## Packaged native acceptance

- Raised the packaged app to the foreground, scrolled Albums near the end, zoomed the native window from a 2560×1600 capture to 3600×2260, and restored its original size.
- The grid changed from five to seven columns and back. Settled covers remained aligned without the stale floating covers seen in the earlier reproduction. The adaptive palette remained consistent.
- Scrolled into the bottom boundary in the wide window. No synthetic viewport stretch or artwork blur appeared.
- Playback remained paused on DON'T TRY at 205.6 seconds throughout; 443 songs, favorites, and collections remained present.
- Background WebKit animation frames can be suspended: an initial background capture appeared blank/frozen despite populated accessibility content. Raising the app resumed rendering. Native visual acceptance was therefore performed in the foreground; background captures alone cannot establish a compositor failure.
- These checks establish native layout/scroll correctness for the observed sizes, not 120 Hz frame pacing or audible audio acceptance.
