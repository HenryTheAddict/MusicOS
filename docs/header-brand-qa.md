# musicOS header and branding repair — 2026-10-27

The supplied Sketch category icon owns a fixed leading action target. Hover or keyboard focus reveals the supplied three-dot glyph over 140 ms without moving the section title. Clicking that target opens app actions; clicking the title or chevron opens destinations. Touch surfaces show the dots directly. Reduced Motion changes glyph states immediately.

The wide selector is 13 rem, with an 0.5 rem gap before Now Playing. Removing the separate trailing action slot gives the player 68 px more width at 100% interface scale. Header icon targets remain at least 44 px at every scale. Compact navigation and the persistent player are retained.

Visible desktop and Android branding is now **musicOS**. Bundle ID, Android package/class names, profile/database paths, keychain and Keystore identities, runtime names, and model files retain their established identities.

Validation:

- 161 UI unit tests and 166 native unit tests passed (one explicit Android fixture remains ignored by default).
- Android unit tests, lint, and signed release packaging passed.
- Header/browser checks cover separate click targets, stationary text, hover/focus glyphs, Reduced Motion, minimum target sizes, available player width, and 480/720/1280 px at all four scales.
- Existing selector checks cover intermediate collapse frames, reversal, replaced-menu focus, opaque connected geometry, and final closed paint. Existing adaptive tests cover retained navigation, Settings surfaces, and persistent player/queue state.

Delivery records, signatures, protected runtime/model hashes, latest profile checkpoint, and fresh playback gate are kept with the release archive. Playback must be paused or stopped before loading; running playback is left untouched.
