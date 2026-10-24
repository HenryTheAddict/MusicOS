# Personal labels, Sketch controls, menu exits and launch appearance

Desktop repair, 24 October 2026. Work stays on `codex/desktop-heart-preservation`;
no OS crates, playback protocol, database schema or installed models changed.

## Result

Personal Home, its collection heading, the sidebar entry/collection box and the
Settings name preview now use the saved display name verbatim. Pending bootstrap
has no invented owner; an empty saved name falls back to “Your library”.

The current Sketch `musicosdesk.sketch` document was inspected through its public
bridge. Its 32 px Play master and album screen confirmed the supplied two-tone
geometry and compact controls. Existing exported geometry is reused. Home creation
and playlist actions use aligned icon/label buttons with 140 ms surface/press
feedback. Suggestions use the supplied Plus vector rather than a text plus.
Collection Play has a solid, artwork-owned tinted plate and readable soft shading;
button and cover bounds stay fixed on hover. Reduced Motion removes movement.

Existing playlists have no M3U action in their ordinary toolbar or Add Music
selector. Import remains in creation/collection management; export is retained in
the shared collection actions menu. Saved sequences are untouched.

Context-menu dismissal retains its pointer origin and presentation coordinates
until the closing panel is removed. Exit animation holds its final transparent
frame, and only the latest panel may restore focus. Selecting an action no longer
cancels the exit with a competing inline animation override. Closed content is
immediately inert.

The native window is created hidden after the saved appearance is loaded. Its
background is chosen before showing it, and the same appearance is injected before
frontend startup. Frontend solid tokens are established before launch interpolation
is enabled. Explicit Dark and System/Dark start dark blue instead of interpolating
from the registered bright-blue defaults. The non-secret browser cache stores only
appearance; saved native preferences take precedence. Existing Light choices remain
Light, and startup does not issue playback actions.

## Verification

- Production UI build and 147 UI unit tests pass.
- 155 native library tests pass; the release build checks the native startup path.
- Browser regressions cover saved name changes, first rendered frame in Dark on a
  Light system, System on a Dark system and explicit Light on a Dark system;
  context-menu opacity/geometry throughout exit, rapid replacement, keyboard focus;
  creation/import placement; fixed Sketch Play geometry and gradients.
- Existing song sliding/morph/reversal, jelly dragging, multi-selection/drop order,
  cancellation, autoscroll, page navigation and integration tests pass.
- Home is inspected at 720×480 and 1280×800, 80/100/125/150% scale, Light/Dark blue,
  artwork and an extreme custom palette. No horizontal clipping was observed.

Browser frame observations establish the absence of a launch palette flash and a
menu exit jump in the exercised cases. They are not a native display FPS claim.
Human motion acceptance and Windows/Linux remain separate gates. The user's saved
appearance is Light/artwork, so native verification retains that choice; conditional
Dark launch is covered by first-frame browser tests and the native initialization
path rather than changing the user's preference for a screenshot.

Signed build identity, full embedded hashes, paused checkpoint and post-load
preservation checks are recorded with the release under
`releases/desktop-name-menu-launch-2026-10-24/`.
