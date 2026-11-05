# Progressive return, Sparklist creation, and lyric word motion — 2026-11-04

## Repair

- Regular album/artist Back retains one inert outgoing markup snapshot. The cover, incoming/outgoing opacity, and returning Filter toolbar use 320 ms with matching easing. Whole-page entry no longer competes with the artwork transfer. Existing scroll, biography state, swipe ownership, reduced-motion, retargeting, and layer cleanup remain intact.
- Filter uses the common 140 ms measured surface/items reveal and reversible exit, stays inert while closing, flips its attachment where necessary, and preserves nested Escape/focus. A stale Radix choice layer caused by an orphaned animation name was repaired.
- Sparklist **creation** is a bounded 40 rem gradient composer, distinct from model setup. Wide composition places artwork beside a compact selector/mood/length group; short layouts retain body scrolling and fixed actions. Step controls are smaller and the whole dialog no longer receives a pointer-open focus border. Model, generation, preview playback, feedback, and explicit saving behavior are unchanged.
- Metadata displays proposed song-field changes rather than empty identical/unreviewed song cards. A summary distinguishes reviewed matches from songs without a reviewed match; unchanged rows remain available. Confidence, source actions, and Ignore are grouped in compact disclosures. Existing cover-only/reviewed-write flows remain unchanged.
- Word lyric foreground fill has a soft moving edge driven by real timestamps. The glow layer persists across words within the active line; native-clock updates drive finite light envelopes including a brief release, without restarting a text animation or introducing a second frame loop. Seeks/RTL/source punctuation remain supported.
- Root `DesignGuidelines.md` records the established design contract, current dimensions, visual/motion rules, and implementation owners.

## Validation

- Full frontend unit suite: **243 passed**, 75 files.
- Browser navigation, metadata, and Sparklist suite: **25 passed**. Includes Sparklist stages at 480×480, 720×480, 1280×800; all four scales; Light/Dark artwork, blue, and extreme custom themes.
- New intermediate-frame/filter/word-motion/metadata checks: **5 passed**. Includes clipped reversal, nested Escape, reduced motion, stable glow identity across word handoff and backward seek, RTL, and no leftover lyric work after closure.
- Existing metadata review/Undo and cover-only isolation checks rerun after the final metadata change.
- Visual screenshots inspected for creation, preview, and metadata; frontend/native production builds passed.
- Physical gesture feel, audible lyric synchronization, and display frame pacing are not newly accepted from browser tests. No new hardware/performance claim is made in this repair.

Installed build, preservation checks, and any native-verification limitations are recorded in the release manifest.

## Installed delivery

- Source: `1d7a3e8a687e9e6a3ffcab91447122d0da7b0f5b`.
- Signed executable SHA-256: `ddcee121e6dc66437bcc80c5ccd22371c9603ace9e03107c85916c9be177cc42`.
- Fresh native paused check preceded installation. The installed app restored the saved song paused at 0:43. Further UI inspection was limited by concurrent user interaction and was stopped; browser checks provide the new presentation evidence.
- Retained 625 tracks, 56 favorites, 2 manual playlists, and 6 manual entries. Manual collection fingerprint and Sparklist sidecar are unchanged.
- Previous app and latest SQLite/preferences checkpoint retained. All eight protected files and primary OS worktree diff are unchanged.
- Release, logs, screenshots, checkpoint pointer, previous app, and manifest: `/Users/henry/Developer/MusicOS/releases/desktop-progressive-return-2026-11-04`.
