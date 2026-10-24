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

- Production UI build and 149 UI unit tests pass.
- 156 native library tests pass; the release build checks the native startup path.
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

## Native startup correction

The first signed candidate was rejected during native launch. The macOS config
replaced Tauri’s window array and omitted manual creation, producing a duplicate
main window. The previous app was restored immediately without restoring an old
profile. The macOS override now carries the same manual-creation/background contract
as the shared config, and a native regression test checks both arrays. The rejected
candidate is retained separately; only the corrected build is a delivery candidate.

WebKit's computed `color(srgb …)` serialization is normalized before deriving the
collection button plate and contrast tones. This corrects a native-only pale-blue
fallback that was visible after the successful launch. RGB, percentage and sRGB
serializations have regression coverage; artwork-owned colors remain cached.

The same plate also retained the launch surface when artwork analysis finished:
its observer watched the shell but the final theme lived on the root. Controls now
observe the owning shell and root and read the explicit target ink for Light/Dark
contrast. Each plate uses its own artwork over a neutral foundation, so another
album changing the page hue cannot recolor it. The regression checks stability
after a page-color change and refreshed surface/contrast after a mode change.

## Final delivery

The corrected signed build was installed after another fresh paused check and clean
quit. SHA-256 of its signed executable:
`860b099c7d320bf8b8949badca1e84057d95e4fc97417332cdc1c24d938c1c98`.
The rejected candidate, previous app and intermediate working app remain preserved.

Native verification confirmed “Henry” in the personal entry, Home and collection
headings; artwork-owned Home button plates; the available Sketch File/Plus glyphs;
no M3U toolbar action on the existing playlist; shared song context actions,
Escape dismissal and restored focus without playback. The background-launched
window was activated before the settled Home screenshot was checked.

Playback remains paused on the same song at its saved 2.0102084 seconds (displayed
0:02). All six collection tables, favorite flags, interface preferences and
integration configuration match the final checkpoint. The library still has
508 songs and 34 favorites. Every embedded runtime/model hash is unchanged.
The app is left open on Home. Native screenshots and test logs are saved under
`out/qa/os-foundation/name-menu-launch/`; full hashes and preservation checks are
in the release record.
