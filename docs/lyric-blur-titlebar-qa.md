# Lyric blur, remaining time, and native titlebar repair — 2026-10-31

Scope: desktop presentation only, on `codex/desktop-heart-preservation`.

- Surrounding timed cues progressively defocus with distance from the acknowledged current cue, capped at 3.2 px. The current cue, hovered/focused cue, and manually scrolled text stay clear. Mini-player blur caps at 1 px. Reduced Motion removes transitions.
- Hover/focus reveals one remaining-time clock. The rail reserves the measured label width plus 8 px instead of a fixed two-clock gap. Full elapsed/total information remains on the accessible seek control. No playback commands change.
- The expanded-player selector uses the measured native traffic-light center line, with a fixed horizontal exclusion for the native buttons. Fullscreen uses its existing edge-to-edge placement. Tabs are compact; action targets remain 44 px.
- Removed a conflicting idle narrow-player grid override that could place song identification on top of lyrics. Content and progress use the same grid whether chrome is visible or hidden.

Validation: TypeScript and production UI build passed; 189 unit tests passed. Browser coverage includes progressive blur/manual reading, titlebar fit at 480 px across 80/100/125/150% scales, one-clock rail spacing, idle chrome, queue/lyrics switching, all mini-player bounds, seek behavior, editor save/Undo and provider retry behavior. Evidence is in `out/lyric-blur-titlebar-2026-10-31/`.

Native traffic-light alignment still requires post-install visual verification. Physical trackpad gestures and Android remain outside this focused repair. No runtime/model, library metadata, preference or sync-schema changes.
