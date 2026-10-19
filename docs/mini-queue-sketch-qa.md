# Sketch queue mini-player and softer Dark mode

The queue presentation follows `musicosdesk.sketch` → BaseUi → Taller Miniplayer (`C1D799DC-B3A1-4474-9E1C-A3A7288A3B0D`), inspected on 2026-10-17. Its 196×256 reference is rendered at 1.25× as a 244×320 native player: thumbnail and left-aligned credits above the queue, then favorite/queue controls and transport. The existing artwork and slim presentations remain available. No drag handle or gradients were added.

Up next opens within the mini-player. Rows use the authoritative queue's entry identities and existing selection, removal, and reordering actions. Duplicate recordings remain separate entries. Escape closes the queue view; the actions menu can restore the main window. Seeking retains rounded caps and accepted DJ transition marks. Native drag excludes the list and interactive controls.

Dark mode keeps neutral charcoal surfaces, with softer primary and secondary text, closer surface tones, subtler borders, and reduced icon contrast. Text contrast remains above 4.5:1 in the tested appearance combinations. Artwork and custom accent preferences are preserved.

## Automated verification

- UI build passed.
- 79 UI unit tests passed, including appearance contrast and mini-menu sequencing.
- 132 native Rust tests passed.
- 23 WebKit tests passed: mini-player layouts, compact queue selection, duplicate-aware removal and keyboard reorder, popup focus, restoration, left alignment, rounded progress, Dark appearance, touch controls, deep scrolling, and queue insertion.
- Screenshot inspection: compact light and dark queue, Dark library, and Dark album controls. Browser fixtures use local sample artwork; native checks use the user's actual queue without editing it.

Screenshots: `out/rebuild-qa/mini-queue-light.png`, `out/rebuild-qa/mini-queue-dark.png`, `out/rebuild-qa/neutral-dark-library.png`.

## Native delivery

Loaded final ad-hoc signed macOS package. Executable SHA-256: `bef35719c0efca6ffb43399880891c5712ebff004323239522970b538b2841b1`. Deep strict signature verification passed. The embedded playback binary and ONNX runtime hashes remain unchanged (`6b1fa66d…` and `ff52828c…`).

Native checks confirmed the compact queue, real artwork, row context menu, themed actions menu, and restoration to the main window. After the final Settings-only adjustment, the nine player/appearance WebKit checks passed again and the final package's mini-player entry and embedded queue were checked natively. Dark Settings was inspected natively before that last adjustment; the final softer selection and native dark color scheme were verified in WebKit. The native display was back in Light mode afterward; its current appearance choice was retained.

Left the final package in its queue mini-player with The Prime Time of Your Life paused at 122.7 seconds and eight upcoming entries. Library count remained 507 songs and 14 favorites. Native QA did not remove or reorder real queue entries. Playback/source-time changes during interactive use are not attributed to this UI change; no exact preservation claim is made across the full session.

Windows/Linux, cross-monitor/fullscreen restoration, measured native drag displacement, and 120 Hz performance remain unverified. This change does not establish audio listening acceptance.
