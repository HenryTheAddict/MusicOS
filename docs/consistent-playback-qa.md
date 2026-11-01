# Consistent playback, navigation and Automix — 2026-11-01

Isolated desktop branch: `codex/desktop-heart-preservation`. Google Drive is excluded as agreed; no Google credentials, cloud UI or Android/Wi-Fi schema changes are introduced.

## Implementation

The expanded player has one dock outside its changing content composition. Seek and transport coordinates stay fixed in Now playing, Lyrics and Up next, with 44 px control targets and a 64 px primary Play target. Lyrics-only layout/button overrides were removed. Chrome reserves its space and changes opacity; it does not move the dock. View selection retains text metrics. Artwork/song composition translates from its displayed position when changing views, while side content uses the same finite 180 ms motion family. Resizing does direct layout reflow. The seek rail retains its measured hover/focus remaining-time slot.

Interactive Back no longer moves the previous-page preview or adds an edge shadow. Current content follows finger movement and the existing release/flick rules; only one inert preview is attached. Snapshots strip media, embedded content and canvases, disable animation, and retain only already-loaded still images. Regular navigation and artwork transfers remain separate from gesture ownership. Mini Player return and native-close paths wait for a finite 140 ms exit, keeping bounds stationary; an interrupted return can be cancelled.

Apple management is an ordinary inline Integrations panel. Recording choices, matched songs and playlist/listening data are disclosures. Existing private sign-in, cached sessions, lyric fallback, background work and transfer validation remain. Primary controls use theme-derived high-contrast fills rather than promotional artwork or a takeover dialog.

A resolved palette snapshot stores appearance identity, resolved system darkness and four validated colors separately from preferences. Native background and pre-React colors consume it only when it matches the current appearance. The renderer presents after applying its initial palette and runs one finite 180 ms / 4 px shell entrance. A two-second native visibility fallback permits diagnosis if the renderer never presents. Startup does not wait for network enrichment and does not start playback. Reduced Motion applies final states directly.

The sample mixer previously applied equal-power gains and then divided by their sum, reducing the energy of an uncorrelated blend. The extra normalization is removed; the existing output limiter bounds peaks. A sample-clock dominant-deck signal switches presentation at the midpoint while retaining the outgoing engine deck until overlap completion. Song identity, source position and listened duration agree; lyric interpolation includes incoming tempo. Explicit seek/skip/configuration changes retain the dominant incoming identity instead of jumping back to the outgoing song. Queue identities and accepted schedules remain protected by existing validation.

Automix keeps phrase-aware 2–16 second planning, pitch-preserving ±6% tempo limits, two-second fallback fades and gapless contiguous albums. Existing defaults arrange unpinned future entries. Automatic analysis now checks only the current/upcoming five tracks, at most once per second. An unchanged failed source is not automatically resubmitted; changed path/size/modification evidence makes it eligible again. Periodic whole-library background analysis was removed.

Downloaded Apple covers have no automatic 256 MiB eviction; motion descriptors have no count-based eviction. An additive recording-scoped asset-link ledger retains the existing paths and cached lyric/catalog association when account index/session data is removed. Disconnect stops work but preserves downloaded assets. Existing files are adopted in place without refetching, renaming, rewriting original covers, or touching original audio. Provider assets remain distinct from rendering intermediates and user lyric revisions.

## Evidence and acceptance

Evidence folder: `out/consistent-playback-2026-11-01/`.

- Production UI build and 194 frontend unit tests passed; 207 native application tests passed, with 7 explicitly live/physical tests ignored.
- The size/theme matrix passed 33 WebKit cases, covering 480×480, 720×480 and 1280×800 at 80/100/125/150% across Light/Dark artwork, blue and extreme custom colors. This includes inline Apple primary-control contrast and Mini Player bounds/menu behavior. Control coordinates also cover individual transport buttons across all player views.
- Functional browser checks cover direct swiping, reversal, momentum, full-distance completion, nested controls, Reduced Motion, retained queue/lyrics, hidden chrome, Apple matching/review and eight-playlist lazy checking. Final run counts are recorded alongside logs.
- Deterministic 48 kHz stereo mixer render: solo RMS 0.141407629, overlap midpoint RMS 0.141407662, peak 0.269569994. Dominant-deck publication occurs at the expected midpoint; original queue identities survive handoff. The WAV and numeric result are retained. This is rendered audio, not physical listening acceptance.
- The core suite passed 93 tests before the added dominant-control case. Latest serial run passed 92/94, including rendered mixing, real decoder transitions, failed incoming files, and dominant identity tests. Two hardware-output/recovery checks failed to observe output progress while macOS was locked. Native automation independently confirmed the Mac was locked. These checks must be rerun with the Mac unlocked; they are not marked passed or ignored.
- Physical trackpad feel, audible installed Automix, output switching and native Mini Player close remain acceptance gates. No 120 fps or hardware-pacing claim is made from browser fixtures.

## Delivery gate

Preserve the latest profile, collection sidecars, original audio, previous app and OS work. Compare all eight protected runtime/model hashes and record the signed executable SHA-256. Installation requires a fresh native paused/stopped check. Do not replace or quit the running app while that check is unavailable or playback is running.

## Installed verification

After the user unlocked and paused musicOS, both previously failing hardware checks passed individually: native output reclamation/reopen and recovery preserving playhead, duplicate queue entries, controls and paused intent. The existing 92 passing serial cases plus these successful reruns cover all 94 current core tests.

The signed update was installed after a fresh native Play-button check on MY EYES at 35.2 seconds. Startup restored that paused track/position. Lyrics loaded, and Up next selected through the shared view controls without issuing playback. The user subsequently interacted with the app and started playback; it was left running. This is not recorded as an audible Automix acceptance test. Physical trackpad and listening acceptance remain outstanding.

Post-install checks verified 550 songs, 40 favorites, 2 manual playlists and 6 entries, collection sidecars, metadata, preferences and user lyric revisions. One newer listening update was retained. All eight protected embedded runtime/model files matched, and the pre-existing primary-checkout guide edit was preserved. Previous app and latest profile checkpoints are retained in `releases/desktop-consistent-playback-2026-11-01`.

Signed executable SHA-256: `cc26e24923778d76ff5cc1cf33c654a87c1a319c4337c0d49899c6fe9547e826`.
