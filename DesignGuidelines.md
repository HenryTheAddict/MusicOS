# musicOS Design Guidelines

This is the design contract for musicOS. It records the established visual language, interaction rules, and the details that must survive future changes. Existing screens that conflict with these rules need repair; their presence in the app does not make them a new design standard.

Use this alongside the current Sketch document and the implementation references at the end. Preserve the user's saved choices. A new feature should feel like another part of musicOS, rather than introduce its own controls, typography, or navigation system.

## 1. Identity and character

- Write the product name **musicOS**. The lowercase `m` is intentional. Keep existing application IDs, profile locations, and compatibility filenames unchanged unless a separate migration is explicitly required.
- The character is playful, colorful, tactile, and personal. Rounded geometry, artwork color, supplied icons, responsive motion, and a small beating heart provide that character.
- Keep the interface direct and useful. Avoid promotional taglines, invented enthusiasm, ornamental instructions, and repeated descriptions of obvious controls.
- Prefer labels such as **Create Sparklist**, **Choose song**, **Preview**, **Save**, **Read more**, and **Follow lyrics**.
- Do not add phrases such as “Meet your mix,” “Playlists you arrange…,” or other marketing copy to working screens.
- musicOS is a product for any listener. Names, paths, defaults, provider identification, and product branding come from shared configuration or saved preferences, never a developer's personal constants.
- Personal headings show the saved display name itself. Do not append “'s stuff,” invent a name during startup, or hardcode Henry. Use **Your library** when there is no saved name. Keep name previews consistent with the actual headings.

## 2. Design sources and ownership

1. Current user instructions and accepted decisions.
2. Current canonical Sketch masters and the supplied desktop, Long Miniplayer, and Tab references.
3. These guidelines and shared presentation components.
4. Existing implementation, where it agrees with the above.

Do not redraw a supplied icon because a handwritten approximation is easier. Do not apply a new design system over musicOS. Inspect the current screen before changing it and inspect the rendered result afterward.

The canonical Library master is `E4FDE931-A4E1-4E34-B419-3560D9BBEB8C`. Its 32×32 geometry and strokes are authoritative. Library, Home, and musicOS branding are distinct icons.

## 3. Typography

- Use the bundled **Figtree** family. `MusicOS Sans` is the implementation's local font-family alias; it is not a second visual direction.
- Text is left aligned by default. Centering is reserved for compact icon controls, balanced player compositions, and explicitly centered shared-voice lyric cues.
- Use a clear hierarchy: page title, section heading, normal label, secondary detail. Large album/artist titles belong to content heroes, not every settings panel or dialog.
- Keep labels readable without making every item bold. Selected tabs and choices must retain identical dimensions and label widths. A weight change must never move adjacent controls.
- Use tabular numbers for times, counts, progress, and changing values.
- Truncate short song/artist identification with ellipsis when necessary. Preserve complete text in detail views, accessible names, and lyrics. Do not truncate a whole editable field's value to solve layout.
- Avoid extremely wide lyric lines, excessive letter compression, and giant text that becomes the entire screen. Lyrics can be expressive and large while keeping current and nearby cues readable.
- Do not scale text during transitions. Animate its position, opacity, light, or clipping instead.

## 4. Color, themes, and images

### Shared roles

Use resolved theme roles rather than fixed per-screen colors:

| Role | Purpose |
| --- | --- |
| `--surface-color` | Primary page or panel surface |
| `--blob-color` | Related raised/selected surface color |
| `--shell-color` | Shell foundation |
| `--ink` | Primary readable text and ordinary icon ink |
| `--secondary-ink` | Readable supporting text |
| `--accent` | Actions, active progress, focus, favorite color where appropriate |
| `--accent-ink` | Readable foreground on the accent |
| `--control-surface` | Soft filled ordinary controls |
| `--control-border` | Quiet contours and separators |

- Retain Light, Dark, System, blue, artwork, and custom themes. Preserve saved saturation and tint values.
- **Light artwork mode is colorful.** At the saved/default 15% tint, artwork color should remain clearly present. Do not neutralize the whole interface to gray to solve a photo or contrast problem.
- Dark surfaces retain an appropriately dark foundation with artwork hue. Do not flash a Light blue shell before resolving Dark/System appearance.
- Primary and secondary text derive from the active surface. Do not hardcode blue status/description text in Integrations or any other screen.
- The palette resolver owns contrast safeguards. Check actual text against page, raised control, selected, and intermediate transition surfaces. Do not bypass safeguards with arbitrary alpha.
- Keep the previous resolved artwork palette while the next one loads. Transition from the displayed palette to the new one. Never flash a default blue or empty palette between songs.
- Cached palettes remain reusable. Ordinary navigation must not trigger repeated extraction, network requests, analysis, or artwork regeneration.

### Gradients and brightness

- Ordinary buttons, inputs, menus, and settings rows use solid theme-colored fills.
- Rich gradients belong to Sparklist artwork/composition, artwork-owned playback colors, and optional immersive Now Playing/lyrics backgrounds. Keep text/control contrast stable over them.
- The optional moving artwork-color background is for **Now Playing and Lyrics**, not a moving background across the whole library or Settings.
- Immersive backgrounds may use broad overlapping color forms and a soft authored ribbon. Motion responds smoothly to existing audio telemetry behind a stable contrast layer.
- Keep all photos, covers, and visualizers at standard SDR brightness. No HDR/EDR boost, washed-out hover filters, or oversaturated photo processing.
- A photo decoding correction must not erase the interface's artwork colors.

### Artwork geometry

- Album artwork is square, bounded, and stable while loading, hovering, navigating, and resizing.
- Prevent grid-row stretching when an album description arrives late. Loading content cannot change a square hero into a tall image.
- Use the supplied smooth-corner geometry. Apply one clipping owner to the artwork, including during transfers.
- Artist portraits and their fallbacks occupy the same full frame. Never place a smaller white export canvas inside an otherwise full-size portrait tile.
- Keep grids still. High-resolution static/animated enrichment belongs to expanded artwork surfaces, with static fallback during delay, error, offline use, Reduced Motion, or unsupported video.
- Only one animated artwork video owns decoding at a time: expanded player, then an open mini-player, then the album hero. It never changes window size or playback state.

## 5. Surfaces and spacing

- Use rounded, connected, solid surfaces with clear hierarchy. Avoid a box inside a box inside another box.
- **No decorative inset shadows or pale inset highlights.** This includes Now Playing, Settings, menus, inputs, and shared controls.
- Quiet outer shadows may separate floating menus/dialogs from content. They must appear and retract with the actual surface, never pop in after its animation.
- Preserve readable borders, group separators, and colored keyboard focus.
- Use compact, consistent spacing. Prefer a small repeated spacing rhythm (4/6/8/12/16/24 px at the relevant presentation scale) over unrelated margins.
- Related settings sit close together; separate groups with a deliberate gap or separator. Do not use giant empty padding as hierarchy.
- Hit targets remain stationary while icons or labels animate inside them. Fullscreen, overflow, and close controls have equal **44 px** targets and optically centered icons.
- Mini-player compact targets are the existing **36 px** controls; do not enlarge its native window to satisfy another surface's layout.
- Visible focus belongs to the focused control. Do not leave a large blue focus outline around an entire dialog after a pointer opening.

## 6. Shared control vocabulary

| Control | Treatment |
| --- | --- |
| Primary action | Filled accent with readable accent ink |
| Secondary action | Compact, rounded, soft theme-colored solid fill |
| Segmented choice | Shared soft container and one stable selected fill; invariant label widths |
| Disclosure | Filled compact action plus one supplied chevron |
| Icon control | Centered supplied icon, consistent target and hover/focus fill |
| Attribution | Actual link, visually separate from actions |
| Destructive action | Explicitly labeled; preserve the existing confirmation/review flow |

- Read more/Read less, Reset theme, and Follow lyrics are buttons with visible soft fills. Do not style them as bare attribution links.
- Reset theme uses the supplied circular-arrow icon and keeps its established reset behavior.
- Album description Read more keeps its dialog. Artist Read more expands in place and keeps Read less at the top.
- Ordinary press response is at most 2% inward. No hover enlargement.
- Preserve the soft two-tone playback geometry, stroke weights, and artwork-owned Play/Sparklist colors. Do not add a mismatched pale button backplate.
- A liked heart is a favorite state, not a selected view. It must not keep a permanent square selection fill.
- Sliders/progress indicators use the active theme. Hover/focus on the seek area reveals remaining time by shortening the visible rail, without moving surrounding controls or changing seek mapping.

## 7. Icons

- Export from the canonical Sketch icon board. Preserve source IDs, dimensions, paths, strokes, two-tone shading, and attribution/export mappings.
- Exclude duplicate screen instances and empty export canvases. Remove accidental canvas backgrounds rather than placing a white square around an icon.
- Use the supplied equivalents for Home, Library, Song, Smart playlist, Plus, Play next, Add to queue, Visualizer, Settings, and fullscreen.
- The supplied sidebar Home glyph is approximately **1.4 rem**, optically centered without enlarging its row.
- Category glyphs can reveal the supplied dots action on hover/focus. Keep the target, row label, and adjacent creation control stationary.
- Visualizer entry uses its supplied Visualizer icon; visualizer options use Settings. Do not substitute the Library icon or an unrelated audio glyph.
- Normalize coordinate systems before morphing. Plus/close, More/close, Shuffle, Play/Pause, and queue/lyrics cycles must not stretch between incompatible exports.

## 8. Desktop shell and navigation

### Wide shell

- Keep native window controls, branding, section selector, app actions, and Now Playing in a compact title area. Give Now Playing useful width rather than leaving large unrelated gaps.
- Keep the sidebar personal entry, Search/Back slot, Albums/Artists/Songs, More, creation access, and saved collections.
- Song count and listening duration share one evenly padded footer outside the collection scroller. No overlapping content or extra bottom void.
- Preserve each destination's history, scroll, filters, selection, and Search state.

### Compact desktop shell

- Use one layout observer for all shell decisions. Do not let CSS, navigation, and player each choose a different breakpoint.
- The current implementation switches below `min(720 px, 40 × effective root-font-size)`; do not restore the older 56 rem proposal independently. A moderately sized square window should retain useful desktop composition.
- Normal desktop minimum is **480×480**. Keep saved bounds and normal restoration from every mini-player preset.
- Compact navigation is **Home · Search · Library**, with one persistent Now Playing row immediately above it.
- Reserve actual layout space for player/navigation/task notices. Lists, dialogs, and editors must not slide behind them.
- Library retains Albums, Artists, Songs, and More as compact category controls. Settings and Devices remain accessible in the section selector.
- Re-selecting the active destination does not reset it. Search keeps its query/results and result-detail Back path.
- Move the persistent player through layout rather than remounting it at a breakpoint. Resizing never issues playback commands or flies controls across the screen.

### Section selector, More, and menus

- Section label stays stationary. Choices attach underneath as one rounded surface.
- Chevron is **down when closed, up when open**. Exactly one transform owner; no combined rotation and flip.
- More, overflow, contributors, context actions, and Filter share the connected-panel family: one outer contour, compact aligned icons, rounded selection, separators between groups.
- Button menus attach to their button. Right-click menus attach near the clicked item/pointer. Flip the attachment edge when needed to remain inside the window.
- Reveal/retract measured, clipped content without scaling text. Rapid toggles reverse the displayed motion.
- Outgoing menus become inert immediately and remain only through their finite exit. They cannot flash on close or restore focus over a newer menu.
- Escape closes the innermost choice/menu before its parent. Preserve keyboard navigation, outside dismissal, action parity, and safe focus restoration.
- Find Tips uses short example/meaning rows, supported literal syntax, and a connected help surface. Escape closes Tips before Find; outside dismissal retains the query.
- Contributor overflow excludes people already shown in chips, uses accurate counts/cached portraits, and scrolls within available height.

## 9. Motion contract

Motion explains where an object went. It must follow the displayed state, remain interruptible, and never delay an action.

| Motion | Established duration |
| --- | --- |
| Play/Pause authored path morph | **360 ms** |
| Cover transfer | **320 ms** |
| Coordinated album/artist return cover, content fade, and Filter-bar handoff | **320 ms** |
| Page/view content, lyric following, skip handoff, Shuffle morph | **180 ms** |
| Menus, ordinary controls, selector fill, mini-player exit | **140 ms** |
| Chrome inactivity timeout | **2.2 seconds** |

### Album and artist navigation

- Capture the displayed cover, preserve its pixels through route commit, restore layout/scroll, then measure the real destination.
- The cover progressively travels back to its actual card. Do not move it to an approximate position and snap at the end.
- One artwork owner controls the transfer. A parent page/album-strip transform cannot also move its anchor.
- Return uses a frozen, inert outgoing page and a coordinated incoming fade. No second unrelated fade or vertical nudge.
- Filter/toolbar reappears progressively as part of the return, rather than appearing instantly above a still-moving cover.
- Preserve expanded artist biography state and resolved content so Back cannot briefly collapse the layout and clamp scroll.
- Disable scroll anchoring where delayed descriptions/artwork would otherwise nudge the page upward.
- A changing destination retargets from the displayed cover. Navigation, scrolling, resizing, unmounting, and Reduced Motion clean up temporary layers.

### Back swiping

- After 12 px of predominantly horizontal movement, content follows the fingers directly. Shell, sidebar/bottom navigation, and Now Playing stay stationary.
- Reveal one frozen, inert previous page at its saved scroll position. No preview parallax, bounce, scaling, delayed cue, or history-wide live mounting.
- Commit at 25% distance or a deliberate fast flick of at least 60 px. Finish within 180 ms or retract within 140 ms from the displayed position.
- Pop history once, ignore trailing momentum, and suppress the regular cover/page transition during interactive swiping.
- Sliders, horizontal shelves, menus, queues, and editors retain their own gestures.

### Playback, drag, and feedback

- Play/Pause uses compatible authored SVG paths, exact Sketch endpoints, fixed footprint, and uninterrupted reversal. Meter/theme/unrelated updates must not restart it.
- Next/Previous directional feedback never delays playback commands. Automix visual identity follows the acknowledged dominant deck once.
- Song Play reveals in the title column. Numbers remain visible while labels slide 1.75 rem over 180 ms. Level bars morph into the playback action on hover/focus.
- Preserve press intent when a control appears under the pointer. Freeze source reveal during dragging.
- Keep the 6 px drag threshold. Only the floating preview gets velocity-driven jelly: at most 3% deformation and 3° tilt, immediate tracking, settle/removal within 180 ms.
- Drop targeting uses actual pointer coordinates. Preserve order, duplicates, queue identity, reorder slots, destination hints, and autoscroll.
- Use one frame owner and coalesced work. Completed, hidden, and settled paused animations leave no continuing frame loops.
- Reduced Motion removes spatial movement, shape morphs, jelly, and animated backgrounds. State changes and playback remain functional.

## 10. Home, library, and detail pages

- Home order: **Continue listening → collections → frequently played albums → mix suggestions → recent songs**.
- Saved track/position is visible while paused. Resume starts only when clicked. Startup remains paused; existing running playback is untouched.
- Collections retain type filters, saved sequences, creation actions, and their existing editors. Do not add destinations simply to surface a creation action.
- Song rows preserve height/columns during hover, selection, click, double-click, context menus, and drag. Action buttons remain isolated from row playback.
- Album heroes keep artwork bounded, contributor chips compact, and metadata readable. With no description, align quality/play/shuffle/more near the bottom of the artwork instead of reserving a large empty description region.
- Artist About is a compact preview when collapsed. At 48 rem of available content width, expanded biography uses roughly two-thirds left text and one-third right portrait/facts/playback. Narrower layouts expand below the header.
- Preserve full biography paragraphs, attribution, Read less, and scroll. Do not stretch text during expansion or place fact stickers over the portrait.

## 11. Expanded player, lyrics, and mini-player

### Shared player

- One shared seek and transport group serves Now Playing, Lyrics, and Up next. Coordinates, gaps, sizes, and hit targets are invariant between views.
- Wide Lyrics/Up next share artwork/song information on the left and content on the right. Now Playing centers its artwork. Narrow views stack content above the same stationary transport.
- No permanent top tab strip. Queue/presentation control cycles **Normal → Up next → Lyrics → Normal** with a supplied-icon morph. Preserve the focused window's keyboard shortcuts.
- Hide chrome after 2.2 seconds; interaction reveals it. Focus, seeking, and open menus hold it visible. Hidden controls are non-interactive.
- Chrome overlays reserved space; it must not push artwork or lyrics around. Keep the visual composition centered.
- Desktop expanded views omit volume; Audio settings already owns it. Mobile volume behavior is separate.
- Native fullscreen is edge-to-edge, without inset frame, divider, margins, rounded border, or backdrop. Restore prior window/fullscreen state on exit.

### Lyrics

- Large, left-aligned readable lines; clear current cues; progressively softer/blurred inactive cues and soft edge masking. Hide the visible scrollbar without disabling manual scrolling.
- Genuine word/syllable timestamps drive progress fill. Ordinary LRC highlights lines. Never invent word timing or hide incorrect editions behind an arbitrary timing compensation.
- Word light eases in/out without rebuilding the layer at every word boundary. Keep foreground ink sharp and wrapping stable; render soft glow outside local text/mask borders so it is not clipped.
- Preserve source punctuation, spaces, Unicode/RTL, overlapping voices, and paragraph structure.
- Lead aligns left, Second voice right, Both centered within the lyric column. Simultaneous voices remain sharp and follow as one stable group.
- Use the acknowledged source clock through seeking, pause/resume, speed, repeat, output changes, and Automix. Timed-line clicks seek; previews of another song do not start playback.
- Manual scrolling suspends follow. Reserve action space for a compact filled Follow lyrics button; it must not cover a line or shift the layout. Mini-player follow sits beside identification.
- Edit lyrics, Find lyrics, Other versions, and attribution details belong in the lyric context menu, available by right-click, keyboard, and touch long-press.
- Distinguish lookup loading from a timed musical break. Use supplied Song icon/three horizontal animated dots appropriately; no literal fake waiting punctuation.
- Provider outages are understandable states, not `[object Object]`, endless refreshes, or an automatic retry loop. Local/cache results remain usable.

### Mini-player

| Preset | Existing minimum | Composition |
| --- | --- | --- |
| Standard | **244×320** | Bounded square cover, song identification, compact controls |
| Compact | **220×120** | Small artwork/identification, controls in one row |
| Wide | **320×120** | More identification width, controls in one row |

- Preserve preset/session bounds. Artwork/Lyrics/Up next switches never enlarge the native window.
- Use the same presentation/lyric components for Appearance previews, without playback actions or new metadata requests.
- Standard artwork must not look like a tiny thumbnail floating in excessive vertical padding. Compact is intentionally small; do not restore a large cover there.
- Preserve dragging, keyboard/pointer resizing, queue scroll/entry identities, pinning, rounded transparency, and menu sequencing.
- Closing runs one finite 140 ms exit before main-window restoration/native close. Rapid reopening cancels it. Playback is unaffected.

## 12. Sparklist creation and setup

These are different surfaces: **creation** chooses a mix; **setup** manages the local model/preparation.

- Creation uses a compact gradient composition, artwork preview, clear starting-song selector, mood choices, and length. Do not make it an almost full-window stack of sparse controls.
- Current creation width is bounded to 40 rem. Wide layouts can place artwork beside controls; short/narrow layouts prioritize controls and reachable actions.
- Steps are **Choose songs → Preview → Name & save**. Use a quiet, compact progress indicator rather than three oversized tabs.
- Starting song and mood remain optional alternatives. Choose for me stays available when songs are ready. Preserve model/analysis gating and the existing generation semantics.
- The primary next action is visually clear and consistently placed. Keep the footer outside the body scroller.
- Preview keeps generated artwork/summary, song list, Regenerate, Less like, and explicit Listen. No automatic playback while composing.
- Save is explicit. Preserve generation choices, name drafts, artwork editing, sequence, and feedback when returning between steps.
- Scroll only where content actually overflows. Fade edges only when there is more content; pad the ends so first/last rows can be read completely. Do not mask fixed actions.
- Model setup uses compact status/progress/actions with expandable diagnostics/licenses. No giant technical paragraphs in the normal creation flow.
- Do not silently replace models, reprocess the library, or regenerate saved collections to improve presentation.

## 13. Settings, Metadata, and onboarding

### Settings

- Preserve existing categories/workflows, visited-page state, drafts, scroll, cached results, and hidden-page inactivity.
- Use compact grouped rows and explicit button roles. Avoid broad button overrides that flatten disclosures, segments, icon controls, and primary actions into one style.
- Sliders and model progress follow the theme. Keep diagnostic detail expandable and secret inputs cleared when inactive.
- Integrations are ordinary settings. Apple is not a large promotional setup. Show connection/capability status, Connect/Reconnect, refresh, save progress/cancel, and compact Disconnect.
- Capability-specific failures do not imply the whole integration is broken. Recording choices, matched songs, and playlists stay expandable.
- Respect clean/explicit recording evidence and explicit source selections. Default preference for unknown lyric editions is Explicit, with clear Clean/Ask and per-song overrides.

### Metadata

- Album browser uses readable full-width rows, with Review and its chevron together. Keep browser beside inspector when space allows; narrow views retain Back/search/scroll.
- Cover-only change is a separate reviewed action and never changes title, album, artist, edition, or song fields.
- Do not render a tall comparison card for every identical song. Show changed fields; summarize matching records and keep **Show unchanged track fields** available.
- Keep match confidence, recording/release sources, and Ignore within compact expandable details. No loose inline buttons colliding with a long confidence sentence.
- Retain review tokens, deliberate selections, revisions, file-writing confirmation/backups, History, and Undo. No automatic field selection.
- Lyrics edits affect the selected song only. Preserve real imported word timing when compatible with the unchanged text.

### Onboarding

- Two focused desktop steps: **Appearance → Music folders**.
- Show a small inert live library preview, optional name, appearance choices, and colors. Do not add a marketing welcome screen or force integrations.
- Folder choice uses the native picker. Original files stay in place. Cancellation is safe; import progress does not block opening the library.
- Phone setup is optional. Existing users are not forced through setup after an update.
- Use defaults/Keep my setup preserves the appropriate saved choices. Saving acknowledges success and provides a readable retry on failure.
- Keep footer reachable at 480×480 and large interface scale; scroll the body only when required.

## 14. Devices and companion design

- Android is a companion in the same musicOS visual language, not a shrunk desktop screenshot or unrelated mobile template.
- Reuse Figtree, supplied icons, colorful surfaces, soft selected fills, coherent player controls, and compact spacing.
- Devices explains current sharing, selected music, preparation/transfer progress, pairing, and paired phones with clear next actions.
- Start/progress feedback must be immediate even when verifying/transcoding takes time. Do not label sharing Off while a task is actively preparing without explaining that state.
- Existing Wi-Fi copying supports offline listening. Keep original and supported Vorbis quality choices clear; avoid technical setup language in the main flow.
- Do not add Google Drive placeholders; that integration was explicitly dropped.
- Native audio/notification/platform acceptance is separate from visual similarity. Do not claim it works solely because a browser preview looks correct.

## 15. Visualizers and haptics

- Use selected preset aliases in the picker while retaining original attribution in Credits/licenses. Settings controls remain compact and near the bottom of Appearance.
- Fullscreen visuals cover the actual viewport. No half-window GPU surface, black mismatch, duplicate player, giant focus rectangle, or framed inner layout.
- Use SDR output, stable lyric contrast, optional album-art colors that interpolate on song changes, and bounded rendering work.
- Keep optional current/next lyrics compact and readable over visuals. Lyrics off means no lookup on visualizer opening.
- Native visualizer ownership, audio tap, preset fidelity, and performance need their own acceptance evidence; never hide unsupported presets behind generic approximations.
- Small hearts beat only for the acknowledged playing song with reliable BPM/phase. Unknown BPM remains still. Do not restart/snap the phase on every telemetry update.
- Trackpad/controller feedback uses one scheduler, supported native APIs, configurable master/beat/interaction switches, and silent fallback on unsupported hardware.
- Heart feedback requires hover over a visible playing-song heart. Stop on pause, exit, track change, hidden windows, or lost telemetry. Cap at four pulses/second by skipping beats, not queueing delayed pulses.
- Drag pickup/valid target/reorder/drop/favorite/toggle/committed slider steps get discrete feedback. Ordinary pointer movement does not.
- Haptics remain independent of Reduced Motion. Physical hardware testing is required before claiming tactile quality.

## 16. Accessibility, performance, and acceptance

- Keyboard, pointer, touch, and controller actions preserve the same outcomes. Keep accessible names correct while glyphs morph.
- Space controls playback outside editors; typing/paste stays native in inputs, lyrics editors, and editable content. Ignore repeated shortcut keydowns where required. One action owner prevents duplicate commands.
- Outgoing/hidden content is inert immediately. Restore focus without scrolling the page or stealing it from a newer menu.
- Use actual pointer coordinates for interaction geometry. Coalesce drag/scroll drawing; avoid repeated layout reads every frame.
- Never repeatedly rescan unchanged library files or refetch resolved data because a page was visited or an animation rendered.
- Measure display pacing, scripting, layout, memory, and idle work. Do not promise 120 fps or assume Rust makes a renderer faster without measurements.
- Validate intermediate frames, rapid reversal, interrupted navigation, delayed artwork/descriptions, resizing, and zero temporary layers/frame work after completion.
- Inspect **480×480, 720×480, 1280×800**, all four interface scales (**80/100/125/150%**), Light/Dark artwork, blue, and extreme custom colors. Include long labels, empty states, slow/offline providers, and every mini-player preset/view.
- Compare refreshed icon exports to Sketch and actual-size rendering. Browser screenshots cannot establish physical trackpad feel, audible Automix quality, or installed GPU fullscreen correctness.
- Deliver from the isolated desktop branch. Preserve the latest profile, collections, previous app, OS work, and protected runtime/model hashes. Record source/build hashes and concrete verification limits.
- Install only after a fresh paused/stopped check. Leave running playback alone. Startup remains paused.

## 17. Implementation map

Paths below are relative to the repository root. Prefer updating these shared owners over adding another override layer:

| Concern | Existing owner/reference |
| --- | --- |
| Product name/defaults | `product.config.json`, `ui-desktop/src/product.ts` |
| Theme/contrast | `ui-desktop/src/appearance.ts`, `artworkTheme.ts` |
| Shell layout | `ui-desktop/src/useShellLayout.ts`, `adaptiveShell.css` |
| Shared icon/control rendering | `ui-desktop/src/components.tsx`, `controls.css` |
| Menu presence and reversal | `ui-desktop/src/menuPresentation.ts`, `menuPresentation.css` |
| Album/artist transfer | `ui-desktop/src/albumTransitions.ts`, `motion.ts`, `presentationMotion.ts` |
| Interactive Back | `ui-desktop/src/useBackSwipe.ts` |
| Expanded player/mini-player | `ui-desktop/src/ExpandedPlayer.tsx`, `MiniPlayer.tsx`, `miniPlayer.css` |
| Acknowledged clock | `ui-desktop/src/playbackFeed.ts` |
| Lyric rendering/follow/glow | `ui-desktop/src/LyricsView.tsx`, `useLyricFollow.ts`, `useLyricGlow.ts`, `lyricWords.ts`, `lyrics.css` |
| Sparklist creation/setup | `ui-desktop/src/SparkCreate.tsx`, `SparkSetup.tsx`, `sparklistPolish.css`, `sparkSetup.css` |
| Metadata | `ui-desktop/src/AlbumMetadata.tsx`, `MetadataWorkspace.tsx`, `management.css` |
| Onboarding | `ui-desktop/src/Onboarding.tsx`, `onboarding.css` |

When changing a shared owner, verify its consumers. Remove competing rules rather than endlessly layering `!important`. Add tests for real behavioral risks and animation continuity; do not substitute test counts for looking at the screen.
