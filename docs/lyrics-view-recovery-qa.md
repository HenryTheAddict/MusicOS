# musicOS lyrics, heartbeat and back-swipe repair — 2026-10-30

Desktop source: isolated `codex/desktop-heart-preservation` branch. Android, OS checkout, playback protocol, saved appearance, original audio and model/runtime resources are unchanged.

## Behavior

- Lyrics: shared client identification, sequential 300 ms request spacing, status-first errors and Retry-After with jitter. Short HTTP 503 backoffs recover at most twice while the view is active. HTTP 429 stays explicitly retryable after the required delay. Successful results remain available offline; errors never become cached misses. Exact title/artist/album/duration matching still controls automatic selection. Broader searches omit the album restriction and present unmatched editions as choices. An invalid optional LyricsFile falls back to valid LRC/plain text.
- Composition: the artwork, song identification, progress and transport form one centered group. Retry explanation and actions stay together. Low-height wide layouts use a smaller bounded cover. Active lyrics follow through one interruptible 180 ms scroll; manual scrolling cancels it.
- Visuals: the scene can use the cached palette of the actual cover even with a blue app theme. Three flowing forms retain a stable contrast veil. Photo decoding remains SDR. The softened canvas is clipped locally. Pause has a finite 180 ms settle; closing releases frames and subscriptions. No Motion button.
- Progress: theme-colored gradient distinguishes the played edge from a quiet rail; time labels use readable theme ink. Seek behavior and targets are unchanged.
- Heart: valid cached BPM sets the period, reliable cached beat evidence supplies an offset, and the shared acknowledged source clock sets phase and playback rate. Seek, speed, repeat and newly visible copies reconcile without a new frame loop. Unknown tempo stays still instead of inventing 72 BPM. Offscreen, paused and Reduced Motion hearts remain still.
- Swipe: previous-page preview now belongs below the moving page inside the same clipped frame. Previously a body-level preview could cover the entire animation because of stacking contexts. First movement paints immediately; subsequent input is coalesced. Release flushes the displayed position, momentum cannot restart navigation, and delayed native phase delivery does not reset a gesture already underway. No additional artwork/page transition runs on commit.

## Verification

- UI unit suite: 182 passed.
- Native unit suite: 187 passed, five intentional external/physical fixtures ignored.
- Focused WebKit interaction suite: 15 passed, including gesture tracking/reversal, momentum, nested controls, Reduced Motion, lyric seeking/editor state, and heartbeat phase after seek.
- Live full native LRCLIB lookup: Helmet / Steve Lacy / Gemini Rights returned ready with timed lyrics; reopening succeeds with an unreachable provider using its successful cache. Before this build, the installed app also successfully loaded Static, confirming the reported failure is intermittent rather than a universally broken transport.
- Deterministic native mock: overload, Retry-After blocking, recovery, exact miss followed by broader search, malformed optional LyricsFile fallback and offline cache reuse passed. Frontend tests cover finite retries and cancellation when the view closes.
- Layout matrix covers 480×480, 720×480 and 1280×800, four interface scales, Light/Dark artwork, blue and extreme custom themes. Results and screenshots are in `out/lyrics-view-2026-10-30/`.
- Timing samples compare the installed source and repaired renderer. Headless WebKit is heavily throttled in this session (roughly 264–301 ms scheduling intervals for both), so these samples cannot establish native display pacing or an FPS improvement. Repaired trials settle without later canvas changes and leave zero canvases on close; unit checks assert no queued work after unmount. Physical native frame pacing and a real two-finger trackpad acceptance check remain hands-on gates.

Build signature/hash, protected resource hashes, latest-profile checkpoint and post-load observations are recorded separately in the delivery's `release.json`. Installation requires a fresh paused/stopped UI check. No claim of physical trackpad acceptance is made from synthetic wheel tests.
