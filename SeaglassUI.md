# Seaglass UI

Seaglass is the design system for the **musicOS handheld OS** (`crates/musicos-init`, the RG35XX / Allwinner H700 build). It replaces the current iPod-style chrome with the look of the musicOS desktop app, and adds a playful layer that borrows from Material You (color pulled from what you are playing, tonal surfaces, big friendly shapes) and from Nintendo system software (a bold, bouncy focus cursor, chunky tiles, button legends, small sounds and moments of delight).

Seaglass is a translation, not a new brand. `DesignGuidelines.md` stays the binding contract for musicOS as a whole; this document says how that contract looks and behaves on a 3.5-inch, 640×480, D-pad-driven screen. Where this document is silent, `DesignGuidelines.md` applies. Where they appear to conflict, the handheld-specific rule here wins only for the handheld, and only for the point it names.

The name: the desktop app's surfaces are soft, rounded, solid and tinted by the artwork, like pieces of sea glass. Seaglass keeps that, and makes it pick-up-and-play.

---

## 0. Why this exists

The current OS UI draws itself as a 2000s portable player:

| Today (`ui.rs`, `ui/theme.rs`) | What it reads as |
| --- | --- |
| `IPOD_CHROME_TOP → IPOD_CHROME_BOTTOM` gray vertical gradients on the status bar and every button | Brushed-metal iPod title bar |
| 2 px `IPOD_BORDER` (`#7e8288`) stroke around every row, panel and button | Skeuomorphic bevelled cells |
| Selected item = blue gradient bar with a darker 2 px outline | iPod/early-iTunes selection |
| `RasterGothic` condensed bitmap-style font | Old LCD firmware |
| Gray `#f4f5f6` page, white rows, `#eff1f3` alternating rows | Spreadsheet list |
| Radii capped at 3–8 px (`RADIUS_SM/MD/LG`, `radius.min(8.0)`) | Square-ish chrome |
| One hard-coded blue (`animated_focus_blue` ignores time) | No relationship to the music |
| Two-pane "Music / Now Playing / Extras / Settings" menu with chevrons | Literal iPod main menu |

The desktop app already has a clear, owned identity: Figtree, large-radius solid tinted surfaces, artwork-derived color, supplied two-tone icons, filled soft controls, no inset shadows, compact rhythm, purposeful motion. Seaglass brings the handheld into that family.

---

## 1. Principles

1. **Same family as the desktop.** A person who uses musicOS on their Mac should recognise the handheld in one glance: Figtree, the musicOS blue, the supplied icons, the soft filled controls, the artwork-colored surfaces, the lowercase **musicOS** wordmark.
2. **Color comes from the music.** The surfaces, focus cursor and controls take their hue from the playing (or focused) album's artwork, like Material You's dynamic color. Without artwork, the user's chosen seed color is used.
3. **Focus is the hero.** There is no pointer. The focused thing is always obvious from across the room, and the cursor that marks it moves, squashes and settles like a physical object (the Nintendo lesson). Everything else stays calm.
4. **Chunky and friendly.** Big radii, big targets, big type for the few things that matter, generous tiles. It should feel like a toy you trust, not a spreadsheet.
5. **Delight in small doses.** Sounds, a bounce, a beating heart, a confetti-free celebration on success. Never in the way of playing a song, never on every frame, always off under Reduced Motion.
6. **One screen, one job.** 640×480 has room for one idea. No box inside a box inside a box; no permanent two-pane menu.
7. **Always know what the buttons do.** A button legend is always on screen, like Nintendo system menus.
8. **Cheap to draw.** The H700 renders in software at 30 fps by default (`MUSICOS_TARGET_FPS`, clamped 15–60). Every effect here is solid fills, rounded rects, one cached blur and cached artwork. No full-screen per-pixel effects per frame.

---

## 2. Platform facts that shape the system

| Fact | Source | Consequence |
| --- | --- | --- |
| Physical framebuffer is 640×480 on RG35XX; code draws in a **1280×960 logical canvas** and `Resolution::offsets()` scales by 0.5 | `renderer.rs` | All values below are given as **logical px (physical px)**. Design at 1280×960, verify at 640×480. |
| 3.5-inch screen, ~229 ppi, held ~30 cm away | Hardware | Body text must be ≥ 36 logical (18 physical) px. Nothing interactive smaller than 88 logical (44 physical) px tall. |
| D-pad, A/B/X/Y, L1/R1, L2/R2, Start/Select/Menu; optional touch on some devices | `input.rs` | Focus-driven UI. Every action reachable by buttons; touch is a bonus, never required. |
| Button meaning: `Confirm`, `Back`, `Context`, `Home`, `Menu`, `PreviousTrack`/`NextTrack` (L2/R2), volume, sleep | `input.rs` `Button` | Legends use these roles. Physical letters follow `MUSICOS_GAMEPAD_LAYOUT` (Anbernic swaps A/B). |
| Software renderer: `rect`, `rounded_rect`, gradients, `shadow_rounded_rect` (layered), `circle`, `draw_svg` (resvg), `draw_image_rounded`, `rusttype` text | `renderer.rs` | Seaglass needs: squircle-ish rounded rects (already), cached SVG rasters, cached blurred artwork, a text cache. |
| Default 30 fps, idle 1 fps redraw | `main.rs` | Motion is designed to look good at 30 fps: fewer, larger moves; springs that settle within ~10 frames. |
| `resvg`/`usvg` already linked | `Cargo.toml` | Use the desktop's supplied SVG icons directly (`assets/icons/ui`). |

---

## 3. Color

### 3.1 Roles

Seaglass uses the same roles as the desktop (`DesignGuidelines.md` §4) so the two platforms can share a palette resolver. In Rust they live in one `Palette` struct (see §13).

| Role | Desktop CSS | Seaglass use |
| --- | --- | --- |
| `shell` | `--shell-color` | Screen background behind everything |
| `surface` | `--surface-color` | Main content sheet (the big rounded card that holds a list) |
| `blob` | `--blob-color` | Raised/selected surface: tiles, focused row fill, sheets |
| `control` | `--control-surface` | Soft filled buttons, chips, track rails |
| `border` | `--control-border` | Hairline separators only (never around every row) |
| `ink` | `--ink` | Primary text and icons |
| `ink2` | `--secondary-ink` | Supporting text (artist, counts, legends) |
| `accent` | `--accent` | Focus cursor, primary action, progress fill, toggles on |
| `accent_ink` | `--accent-ink` | Text/icons on accent |
| `hero` *(new)* | — | Saturated artwork color for Now Playing backgrounds and Home's "playing" tile only |

### 3.2 Where color comes from

Exactly like the desktop's `themePalette()` (`ui-desktop/src/appearance.ts`):

1. **Artwork mode (default).** Extract `background`, `blob`, `accent` from the current track's cover once, when the track changes (the OS already decodes `CoverArt`). Cache the palette with the cover. Never re-extract on navigation.
2. **Seed mode.** If there is no artwork or the user chooses a fixed color, use a seed. Existing `ThemeName` values become seeds:

   | `ThemeName` | Seed | Note |
   | --- | --- | --- |
   | `Blue` | `#0064df` (musicOS blue, the desktop `--accent`) | Default. Replaces `rgb(35,108,220)` / `rgb(37,112,190)`. |
   | `Red` | `#cf3e52` | Kept from `primary_color()` |
   | `Green` | `#1e955c` | Kept |
   | `Purple` | `#895bc4` | Kept |

   Add **Artwork** as a fifth choice and make it the default for new devices (matches desktop "From artwork").
3. **Light / Dark / Auto.** Port the desktop algorithm literally so both platforms resolve the same palette for the same album:
   - Light: `surface` = mix(`#f7f9fc`, artwork background, tint/15), `blob` = mix(`#e6ebf4`, artwork blob, tint/15), ink `#12202a`; lift toward white until contrast ≥ 4.8:1.
   - Dark: `surface` = mix(`#202124`, tone, tint×0.33), `blob` = mix(`#2b2d32`, tone, tint×0.40), ink `#dedfe3`; darken until ink contrast ≥ 7:1; lighten accent until ≥ 4.5:1 on both.
   - Default tint **15 %**, saturation **100 %**, like the desktop. Light artwork mode stays visibly colorful.
   - Auto follows the clock (dark 19:00–07:00) since the device has no system appearance.
4. **Shell.** `shell` = `surface` darkened 6 % in Light, `surface` darkened 25 % in Dark. The shell shows as a frame around the content sheet and under the status bar.
5. **Hero.** `hero` = artwork accent at full saturation, used only as the Now Playing backdrop and the Home "Now Playing" tile.

### 3.3 Transitions between palettes

- When the track changes, cross-fade every role from the displayed palette to the new one over **320 ms** (same as desktop cover transfer). Interpolate in linear RGB.
- Keep the previous palette while the next cover decodes. Never flash the default blue between songs.
- On first boot before any track, use the seed palette.

### 3.4 Contrast and safety

- Text on any surface ≥ 4.5:1; big titles (≥ 56 logical px) ≥ 3:1. The resolver enforces it; screens never apply arbitrary alpha to "fix" contrast.
- Focus cursor vs. the surface behind it ≥ 3:1 (WCAG non-text).
- Monochrome artwork keeps a neutral foundation; do not invent a blue.
- Covers and photos stay at their own brightness. No filters on artwork.

### 3.5 Banned

- Vertical gray "chrome" gradients (`IPOD_CHROME_*`) anywhere.
- Gradients on ordinary buttons, rows and menus. Solid fills only (desktop §4 "Gradients and brightness").
- Alternating row stripes (`IPOD_ROW_ALT`).
- Pure `#000` text on pure `#fff` page as the whole look.

---

## 4. Typography

### 4.1 Typeface

- **Figtree**, the desktop family (`assets/fonts/Figtree-Variable.ttf`). Replace `RasterGothic` as the UI font.
- `rusttype` does not read variable-font axes, so ship **static instances** generated once from the variable file (for example `fonttools varLib.instancer`): **Figtree-Regular (400)**, **Figtree-SemiBold (600)**, **Figtree-Bold (700)**, **Figtree-Black (900)**. Keep the OFL notice (`Figtree-OFL.txt`).
- Keep **Noto Sans JP / SC** as fallbacks for CJK, as today.
- `FontWeight` grows from `Regular | Bold` to `Regular | SemiBold | Bold | Black`.
- RasterGothic may stay as an *easter-egg* "Classic" option in Settings, never the default.

### 4.2 Scale

Sizes in logical px (physical in brackets). Line height ≈ 1.2× for titles, 1.3× for body.

| Token | Size | Weight | Use |
| --- | --- | --- | --- |
| `display` | 88 (44) | Black | Now Playing song title on idle/large layout, setup headings |
| `title` | 64 (32) | Black | Page title ("Albums", "Settings") |
| `headline` | 48 (24) | Bold | Album title in a detail header, dialog titles, Home tile labels |
| `body-strong` | 40 (20) | SemiBold | List row primary line, button labels |
| `body` | 36 (18) | Regular | List row secondary line, settings values |
| `caption` | 30 (15) | SemiBold | Status bar, button legends, counts, times (minimum text size) |

- Titles use the desktop's character: heavy (Black 900), slightly tight (-1 % tracking), left aligned.
- Times, counts, battery and progress use tabular figures. `rusttype` does not apply OpenType features, so lay digits out on a fixed advance (the widest digit) rather than relying on `tnum`.
- Truncate with an ellipsis (`text_fit`), never by clipping mid-glyph. A focused row whose title is truncated **marquees** after 800 ms (scroll at 60 logical px/s, pause 1 s at each end). This is the one place text moves.
- Do not scale text during transitions; move or fade it.
- The lowercase **musicOS** wordmark appears in setup and About only, never in the status bar (the current "MusicOS" status-bar title goes away).

---

## 5. Shape

Seaglass is round. Radii are about 3× today's.

| Token | Radius | Use |
| --- | --- | --- |
| `r-xs` | 12 (6) | Progress rail, small chips |
| `r-sm` | 20 (10) | Thumbnails in rows, toggles' tracks |
| `r-md` | 32 (16) | List rows, buttons, menu items |
| `r-lg` | 44 (22) | Tiles, content sheet, artwork in grids |
| `r-xl` | 64 (32) | Now Playing artwork, sheets/dialogs, HUDs |
| `r-pill` | h/2 | Focus cursor on single-line rows, legends, segmented controls |

- Corners use the existing `rounded_rect`; where cheap, approximate the desktop's smooth-corner squircle by extending the corner by 1.2× radius with a superellipse mask that is computed once per size and cached.
- Artwork is always square with `r-lg` (grids) or `r-xl` (Now Playing). Artist pictures are circles.
- One clipping owner per artwork, including during transfers.

---

## 6. Space and layout

### 6.1 Rhythm

Base unit **8 logical (4 physical) px**. Allowed steps: 8, 12, 16, 24, 32, 48, 64 logical — the desktop 4/6/8/12/16/24/32 rhythm doubled.

### 6.2 Screen frame

```
┌──────────────────────────── 1280 ────────────────────────────┐
│ status bar  64 (32)  time · now-playing glyph · battery       │  shell
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ content sheet  x=16 y=72 w=1248 h=808  r-lg  surface      │ │
│ │  page title 64 + 24 gap                                   │ │
│ │  content (list / grid / player)                           │ │
│ └──────────────────────────────────────────────────────────┘ │
│ legend bar  80 (40)   Ⓐ Play  Ⓑ Back  Ⓧ More    ♪ mini     │  shell
└───────────────────────────────────────────────────────────────┘
```

- **Status bar** 64 (32) tall, on `shell`, no gradient, no border. Left: time (caption, tabular). Center: nothing, or a 24 px animated level glyph when audio plays. Right: volume glyph when muted, battery pill (outlined `r-pill` with accent fill when charging, red only below 10 %).
- **Content sheet** inset 16 logical on the sides, `r-lg`, `surface` fill, no stroke. On Home and Now Playing the sheet is replaced by the screen's own composition.
- **Legend bar** 80 (40) tall at the bottom on `shell` (see §9.4). Its right side can show the **mini now-playing chip** (cover + level glyph) when you are away from Now Playing.
- Content padding inside the sheet: 32 logical. Gap between list rows: 8 logical. Gap between sections: 48 logical.

### 6.3 Targets

- Focusable rows ≥ 104 (52) tall. Grid tiles ≥ 256 (128). Icon buttons 88 (44) square, matching the desktop's 44 px rule.
- With touch present, the same sizes satisfy touch.

---

## 7. Elevation and surfaces

- Hierarchy comes from **tone**, not shadows: `shell` < `surface` < `blob` < `accent`. That is the Material You tonal elevation idea, and it matches desktop's solid surfaces.
- **No inset shadows, inner highlights, or "liquid glass"** (`inner_shadow`, `liquid_glass`, `liquid_glass_rounded_rect` stop being used for UI chrome). Desktop §5 bans decorative inset shadows.
- **One quiet outer shadow** (`shadow_rr`, 24 logical blur, 12 % black, y+8) for floating things only: sheets, HUDs, the context menu, the focus cursor when it lifts during a press. It animates with its surface.
- No strokes around rows, buttons or panels. Separators are `border` hairlines 2 (1) px, inset 32 from both sides, between groups only.

---

## 8. Iconography

- Use the desktop's supplied icons (`assets/icons/ui/*.svg`: `Home`, `Library`, `album`, `artist`, `song`, `playlist`, `queue`, `search`, `settings`, `play`, `pause`, `next`, `previous`, `shuffle`, `repeat`, `repeat-one`, `favorite`, `volume`, `DJmode`, `Lossless`, …) and `assets/icons/nowplaying` for transport.
- Icons are tinted at draw time with `ink` (or `accent_ink` on accent), preserving their two-tone shading where the SVG has it. Cache each tinted raster by `(path, size, color)`.
- Sizes: 32 (16) inline, 48 (24) in rows and legends, 64 (32) buttons, 96 (48) Home tiles, 128 (64) transport Play.
- Retire the `assets/icons/appicons` iPod-style menu set (`Music.svg`/`MusicSelected.svg`, etc.). Selected state is shown by the cursor, not a second icon.
- Do not hand-draw icons in Rust (`draw_globe` stays only for the Language screen illustration).

---

## 9. Focus, input and feedback (the Nintendo layer)

### 9.1 The cursor

The focus cursor is one object, owned by the screen, that **travels** between focusable items.

- **Shape:** a filled `blob`-to-`accent` pill behind the focused item plus a 6 (3) px `accent` ring with a 4 (2) px gap (`focus_halo`, redrawn with Seaglass tokens). Rows: filled pill. Tiles and artwork: ring only, so the artwork stays visible.
- **Travel:** when focus moves, the cursor's rect animates from the old item to the new one with a spring (stiffness 520, damping 32 → settles in ≈ 180 ms, ≤ 6 % overshoot). This replaces the current cosine `Position::interpolate` and is the single most important "feel" change.
- **Squash:** on the frame focus leaves, the cursor stretches 6 % along the direction of travel and returns to 100 % as it lands. Content never scales; only the cursor does.
- **Breath:** an idle focused ring pulses its ring alpha 70 %→100 % over 1.6 s (the Switch's "you are here"). Pauses during input and under Reduced Motion.
- **Edge bump:** pressing past the first/last item nudges the cursor 12 logical px in that direction and springs back (140 ms), with the "bonk" sound. Lists do not wrap unless the screen says so (Home tiles do wrap).
- **Press:** `Confirm` presses the focused item 2 % inward for 80 ms (the desktop's maximum press response), then the action happens. The action is never delayed by the animation.

### 9.2 Scrolling

- Lists keep the cursor in the middle third. The list scrolls under a stationary cursor once it reaches that band (iPod-style *content* scroll, not cursor scroll), with the same spring.
- Holding a direction repeats at 6/s after 350 ms, accelerating to 15/s after 1.5 s. After 2 s on a sorted list, a large letter bubble (`display`, `r-xl`, `blob`) shows the current initial, like a jump-scroll.
- L1/R1 page up/down by one screen of rows. On sorted lists they jump to the previous/next letter.
- A thin scroll indicator (8 logical wide, `r-pill`, `ink2` at 40 %) appears on the right edge only while scrolling and fades after 600 ms. Replaces the permanent `ipod_scrollbar`.

### 9.3 Button roles

| Role | Default action | Never |
| --- | --- | --- |
| `Confirm` | Open / play the focused item | Destructive actions without a confirm sheet |
| `Back` | Go back one level; on Home, does nothing (bonk) | Exit playback |
| `Context` | Open the context sheet for the focused item | — |
| `Home` | Go to Home from anywhere (playback continues) | — |
| `Menu` | Quick settings sheet (volume, brightness, shuffle, repeat, sleep timer, theme) | — |
| `PreviousTrack` / `NextTrack` | Skip, anywhere | — |
| Play/Pause key | Toggle playback, anywhere | — |

### 9.4 Legend bar

- Always visible at the bottom (except fullscreen Now Playing idle, where it fades with the chrome after 2.2 s like the desktop expanded player).
- Each legend: round button glyph (48 logical circle in `control`, letter in Bold `caption`) + verb in `caption` SemiBold `ink2`. Spaced 32 apart, left aligned.
- Glyph letters match the physical layout (`MUSICOS_GAMEPAD_LAYOUT`), so on Anbernic `Confirm` shows **A** on the right face button.
- Verbs are specific: **Play**, **Open**, **Back**, **More**, **Shuffle**, never "OK"/"Select".
- When a legend's action changes (e.g. focused row is a song vs. an album), cross-fade the verb (140 ms); glyphs stay put.

### 9.5 Sound

Short UI sounds are part of the character. All are optional (Settings → Sounds: On / Quiet / Off; default Quiet), mixed under music at −18 dB, never while scrubbing.

| Event | Sound |
| --- | --- |
| Focus move | Soft "tick" (pitch rises 1 semitone per step within a 6-step run, resets after 400 ms idle) |
| Confirm | Two-note "pop" |
| Back | Lower "pop" |
| Edge bump | Muted "bonk" |
| Sheet open / close | Airy "swish" up / down |
| Favorite | Little heart "ding" |
| Error | Two low notes |

### 9.6 Rumble

Devices with rumble: 20 ms light pulse on edge bump and favorite only. Respect the same master/beat/interaction switches as desktop haptics (`DesignGuidelines.md` §15). Silent on unsupported hardware.

---

## 10. Motion

The desktop motion contract applies, re-timed for 30 fps where noted.

| Motion | Duration / curve |
| --- | --- |
| Focus cursor travel | Spring k=520, c=32 (≈ 180 ms) |
| Page push (into a list/detail) | 180 ms; incoming slides 64 logical from the right + fades in, outgoing fades + slides 32 left. `ease-out` cubic-bezier(.2,.7,.2,1) |
| Page pop (Back) | 180 ms mirror of push |
| Cover transfer (grid tile → album header / Now Playing artwork) | 320 ms, same artwork pixels travel; one owner |
| Palette cross-fade | 320 ms |
| Sheets (context, quick settings) | Rise from the bottom 140 ms with a 4 % overshoot; close 140 ms without overshoot |
| HUD (volume/brightness) | Appear 140 ms, hold 1.5 s after last change, leave 180 ms |
| Play/Pause glyph morph | 360 ms authored path morph (reuse desktop SVG paths) |
| Legend verb change, toggles, selection fills | 140 ms |
| Now Playing chrome hide | after 2.2 s idle |

- Motion is interruptible: a new input retargets from the displayed position.
- Prefer one large movement to several small ones. At 30 fps, anything under 100 ms is just a jump; use instant state change instead.
- **Reduced Motion** (Settings → Accessibility, mirrors desktop): no travel, squash, breath, overshoot, marquee or backdrop motion. Cursor jumps; pages cross-fade 140 ms; sounds and rumble stay governed by their own setting.

---

## 11. Components

Every component below is a Rust function in `crates/musicos-init/src/ui` that takes `&Palette`/`&Tokens` rather than literal colors.

### 11.1 Status bar
See §6.2. Height 64 (32). Time left, battery right. No title, no gradient, no bottom border.

### 11.2 Content sheet
`surface`, `r-lg`, inset 16. Page title in `title` at (48, 104). Optional subtitle (`body`, `ink2`) 8 below.

### 11.3 List row
- 104 (52) tall single line, 128 (64) with two lines. `r-md`.
- Leading: 80 (40) artwork `r-sm`, or 48 icon in a 80 `control` circle, or a track number in `body` tabular `ink2`.
- Text: primary `body-strong` `ink`; secondary `body` `ink2`.
- Trailing: one of duration (`caption` tabular), a chevron (32, `ink2`), a toggle, a value (`body` `ink2`), a playing-level glyph, a heart.
- Unfocused rows have **no fill and no stroke**. Focused row gets the cursor pill (`blob` mixed 30 % toward `accent`), text stays `ink`.
- The playing track shows an animated 3-bar level glyph in `accent` in place of its number.

### 11.4 Tile
- Home and grid building block. `r-lg`, `blob` fill or full-bleed artwork.
- Label below in `headline` (Home) or `body-strong` (grid), secondary in `body` `ink2`.
- Focused: cursor ring + lift (shadow appears, tile moves up 8 logical). The tile does not scale.

### 11.5 Buttons
| Kind | Look |
| --- | --- |
| Primary | `accent` fill, `accent_ink` label, `r-pill`, 104 tall |
| Secondary | `control` fill, `ink` label, `r-pill`, 88 tall |
| Icon | 88 square, centered 48 icon, `control` fill only when focused |
| Destructive | Secondary with label in the resolver's danger color, always followed by a confirm sheet |

### 11.6 Toggle, segmented, slider
- Toggle: 96×56 track `r-pill` in `control` (off) / `accent` (on), 44 knob in `surface`, knob travels 140 ms.
- Segmented: one `control` pill containing options; the selected option is a `surface` pill that slides between options (140 ms). Options have identical widths.
- Slider (volume, brightness, EQ): 16 tall rail `r-pill` `control`, `accent` fill, 40 knob only when focused; Left/Right step 5 %, L1/R1 step 25 %.

### 11.7 Progress / seek
- 16 (8) tall rail `r-pill` in `control`, filled `accent`. Elapsed left, remaining right in `caption` tabular.
- When the seek bar has focus, the rail grows to 24 and a 40 knob appears; Left/Right seek 5 s, hold to accelerate, L1/R1 ±30 s.

### 11.8 Sheet (context menu, quick settings, confirm)
- Rises from the bottom, full width minus 32, `r-xl` top corners, `surface` fill, quiet shadow; content behind dims with `shell` at 50 %.
- Header: item artwork 96 `r-sm` + title `headline` + subtitle `body` `ink2`.
- Actions are list rows with leading icons: **Play**, **Play next**, **Add to queue**, **Shuffle**, **Go to album**, **Go to artist**, **Favorite**, **Rate**. Destructive rows last, after a separator.
- `Back` or `Context` again closes it. Replaces `draw_context_menu`.

### 11.9 HUD (volume, brightness)
- Pill 480×120 centered near the top, `blob` fill, quiet shadow, icon 64 + slider. Replaces the current HUD styling; same timings as §10.

### 11.10 Toast
- Pill at the top under the status bar, `accent` fill, `accent_ink` caption, for "Added to queue", "Saved", "Library updated". 1.8 s.

### 11.11 Empty state
- Supplied icon at 128 in a 224 `blob` circle, `headline` title, `body` `ink2` line, optional primary button. One per screen, centered in the sheet. No outlined white card (the current Now Playing "No song selected" box goes away).

### 11.12 On-screen keyboard (Search)
- Grid of `r-md` keys, 96×96, on `surface`. Focused key gets the cursor. Space/Delete/Clear wider. Results preview above the keyboard updates live. Y types a space, X deletes (legend shows it).

---

## 12. Screens

Each maps to a value of `app::Screen`.

### 12.1 First / Welcome / Theme / DateTimeSetup / Language / TestComplete (setup)
- Full-screen `hero`-tinted composition, lowercase **musicOS** wordmark in `display`, one primary button. Theme step lets you pick a seed (Artwork, Blue, Red, Green, Purple) and Light/Dark/Auto with a live mini-preview of Home, exactly like desktop onboarding's live preview.
- Two-step feel like desktop onboarding: **Appearance → Music**. Language and date/time are compact rows, not separate big screens where avoidable.
- Keep the 640×480 half-scale relationship from `docs/figma-target.md`; the iPod-style frames there are superseded by this document.

### 12.2 Home
Nintendo-style row of big tiles instead of the iPod two-pane menu.

- Top: greeting/user name in `title` (desktop: saved display name, else **Your library**).
- Middle: a horizontal row of **tiles** 320×320 (160): **Now Playing** (current cover, `hero` backdrop, level glyph), **Albums**, **Artists**, **Songs**, **Playlists**, **Search**, **Settings**. Left/Right moves; the row scrolls so the focused tile stays one slot from the left. Wraps.
- Below the row: one **Continue listening** row (cover, title, artist, Resume), visible when something is paused.
- Legend: Ⓐ Open · Ⓧ More · Ⓨ Search.
- The "Extras/Apps" entry folds into Settings → Extras unless it hosts more than one real app.

### 12.3 MusicApp (library root)
If kept as a screen: a 2×3 grid of category tiles (Albums, Artists, Songs, Playlists, Genres, Search). Otherwise Home tiles go straight to the lists and this screen is removed.

### 12.4 AlbumsList / ArtistsList / ArtistAlbumsList
- Albums: **3-column cover grid**, tiles 352 (176) square, title + artist below. Left/Right/Up/Down move the cursor; L1/R1 jump letters. `Context` opens the album sheet.
- Artists: list rows with 80 circular portraits (fallback: initial in a `blob` circle).
- ArtistAlbumsList: artist header (portrait 160 circle, name `title`, counts `body`) above an album grid.

### 12.5 AlbumTracksList / PlaylistTracksList
- Header (not a separate box): artwork 240 `r-lg` left, title `headline`, artist `body-strong`, year · count · length `body` `ink2`, then **Play** (primary) and **Shuffle** (secondary) buttons. Cursor starts on Play.
- Tracks: rows with numbers, durations, playing glyph, heart.
- Cover transfer from the grid tile into this header (§10).

### 12.6 SongsList
- Rows with 80 artwork, title, artist · album, duration. Sorted with letter bubbles.

### 12.7 NowPlaying
The showpiece.

- Backdrop: the current cover, downscaled to 64×48, blurred once per track, scaled up and laid under a `hero` → `shell` tint at 70 %. Cached; no per-frame blur. Optional very slow drift (≤ 2 px/s) of the backdrop, off under Reduced Motion.
- Left: artwork 560 (280) square, `r-xl`, quiet shadow. Right: title `display`/`headline` (fits 2 lines, then marquee), artist `body-strong`, album `body` `ink2`, heart.
- Bottom: seek bar (§11.7) and transport: previous / **Play-Pause** (128 circle, `accent` fill, `accent_ink` glyph, authored morph) / next; shuffle and repeat as icon buttons at the ends.
- Up/Down move focus between transport, seek and the info row; Left/Right move within the transport. L2/R2 skip anywhere.
- Small heart next to the title beats with the song's BPM only when BPM is known and the song is playing (desktop §15).
- Chrome (legend, status bar) fades after 2.2 s idle; any input brings it back.
- `Context` opens the song sheet; Ⓨ opens Up Next (Queue).

### 12.8 Queue (Up Next)
- Sheet-like page with the playing song pinned at the top (`blob` card), then upcoming rows. `Context` on a row: Play now, Move up, Move down, Remove. Reordering moves the row with the cursor (pick up with Ⓐ, move, drop with Ⓐ).

### 12.9 Search
- Keyboard (§11.12) with live results grouped Songs / Albums / Artists above it. Results become focusable with Up from the keyboard.

### 12.10 RatingPicker
- Five large hearts (or stars, matching the desktop rating glyph) in a row inside a sheet; Left/Right sets, Ⓐ saves. Each step plays the tick with rising pitch.

### 12.11 ContextMenu
- §11.8 sheet.

### 12.12 Settings / SettingsDetail
- Grouped rows like the desktop's compact grouped settings: **Appearance** (Theme: Artwork/Blue/Red/Green/Purple, Mode, Tint, Reduced Motion, Classic font), **Sound** (EQ, Volume limit, UI sounds), **Playback** (Shuffle, Repeat, Gapless, Sleep timer), **Library** (Rescan, Storage), **Device** (Brightness, Date & time, Language, Name), **About**.
- Group titles `caption` SemiBold `ink2` uppercase-free, 48 above the group. Values right-aligned.
- Changes apply immediately with a live preview (theme changes cross-fade).

### 12.13 Apps / About
- Apps: tile grid if there are several apps, otherwise fold into Settings.
- About: wordmark, version, storage, song count, device name, build hash; one tile per fact, no paragraphs.

---

## 13. Implementation map

Seaglass is mostly a refactor of shared helpers, not of every screen at once.

| Concern | Today | Seaglass owner |
| --- | --- | --- |
| Tokens | `RADIUS_*`, `IPOD_*` consts in `ui.rs`; `ThemeName::primary_color/gradient_colors` | New `ui/seaglass.rs`: `Palette { shell, surface, blob, control, border, ink, ink2, accent, accent_ink, hero }`, `Radius`, `Space`, `Type` consts, `Motion` consts |
| Palette resolver | Fixed per-theme colors | `seaglass::resolve(appearance, artwork_palette, is_dark) -> Palette`, a direct port of `themePalette`/`secondaryInk` from `ui-desktop/src/appearance.ts`, with unit tests mirroring `ui-desktop/tests/appearance.test.ts` |
| Artwork palette | — | Extract on track change next to `CoverArt` decode; cache alongside the cover |
| Fonts | `assets.rs` RasterGothic | Figtree static instances + Noto fallbacks; `FontWeight` gains `SemiBold`, `Black` |
| Rows, buttons, panels | `figma_list_row`, `button_with_radius`, `panel_gradient`, `figma_split_panels` | `seaglass::row`, `seaglass::button`, `seaglass::sheet`, `seaglass::tile` |
| Focus | `focus_halo`, `register_selected`, `transitions::Position::interpolate` | `seaglass::Cursor` (spring, squash, breath) owned by each screen |
| Status bar | `ipod_status_bar` | `seaglass::status_bar` |
| Scrollbar | `ipod_scrollbar` | `seaglass::scroll_indicator` (transient) |
| Context menu | `draw_context_menu` | `seaglass::sheet` + rows |
| Legends | — | `seaglass::legend_bar(&[(Button, &str)])` |
| Sounds | — | Small PCM clips through the existing audio path, mixed under music |

**Migration order** (each step shippable on its own and visible in `musicos-snapshot`):

1. Tokens + palette resolver + Figtree. Swap `IPOD_*` uses for palette roles; remove gradients/borders. *(Biggest visual change for least code.)*
2. Status bar, content sheet, list row, legend bar, cursor travel.
3. Home tiles, Albums grid, album header with cover transfer.
4. Now Playing composition, transport morph, backdrop cache.
5. Sheets (context, quick settings), HUDs, toasts, empty states.
6. Setup flow, Settings regrouping, sounds, rumble, Reduced Motion.

---

## 14. Verification

- Render every screen with the snapshot tool at both sizes and both modes:

  ```sh
  cargo build -p musicos-init --bin musicos-snapshot
  for s in welcome theme home music albums queue context now settings apps about volume-hud brightness-hud; do
    MUSICOS_MEDIA_DB=/tmp/snap.sqlite3 MUSICOS_SETTINGS_PATH=/tmp/snap.json \
    MUSICOS_RESOLUTION=640x480 MUSICOS_SNAPSHOT_SCREEN=$s \
      ./target/debug/musicos-snapshot out/seaglass-$s.bmp
  done
  ```

  (`MUSICOS_SNAPSHOT_TRANSITION` and `MUSICOS_SNAPSHOT_SECONDS` capture mid-transition frames.)
- Check each at Light and Dark, Artwork with a colorful cover, a monochrome cover and no cover, and each seed.
- Long titles (60+ chars), CJK titles, empty library, 5 000-song lists, queue of 500.
- Contrast: run the resolver tests; spot-check focused row text and legend text.
- Performance on the device: 30 fps sustained while scrolling a long list and while the cursor travels; idle stays at 1 fps. Measure, do not assume.
- Reduced Motion on: no movement except instant focus and fades.
- On hardware: legend letters match the buttons on the RG35XX layout; sounds are audible but under the music; QEMU cannot prove audio, rumble or feel.

---

## 15. Open design calls

Defaults are chosen above; these are the ones worth Henry's eye.

1. **Name**: *Seaglass UI* (default) for the handheld design language.
2. **Home**: Nintendo-style horizontal tile row (default) vs. a vertical list of big rows.
3. **Default color**: Artwork (default, matches desktop "From artwork") vs. musicOS blue.
4. **UI sounds**: on at "Quiet" by default (default) vs. off by default.
5. **Albums**: 3-column cover grid (default) vs. 2 columns with bigger covers.
6. **Classic font**: keep RasterGothic as a hidden "Classic" option (default) vs. remove it.
7. **Extras/Apps**: fold into Settings (default) unless more apps are planned.
