# Junk cover and menu/playback repair — 2026-10-26

Junk's FLAC files contain no cover. A readable 1200×1200 front image was already
cached for its MusicBrainz suggestion (e8ea9cb0-19ed-40eb-b25e-081092c2eeb2), but
had not been selected. The requested cover repair assigns only this artwork to
its 15 songs through the existing undoable metadata API (history 9). Pending
metadata comparisons remain unchanged. All 15 source audio SHA-256 hashes and
raw track fields excluding the expected revision increments match the private
pre-repair checkpoint. No download, tag write or model processing was performed.

Menus now retain their 140 ms WAAPI timelines through a reversal instead of
cancelling and creating new curves from rest. The same timeline runs backwards
for retraction; text is clipped rather than scaled. Endpoint styles are committed
before cancelling the completed effects. Panel scrolling does not finish an
animation; outside scrolling dismisses through the exit rather than truncating
one already in progress. Resize and Reduced Motion retain immediate cleanup.
Each opening also has its own focus session, so an old deferred focus return
cannot steal focus from a newer opening of the same selector.

Pointer menu interaction no longer turns a subsequent Space press into button
activation. Its programmatic focus return remains accessible, without a spurious
Tab outline. Tab, Enter and arrow navigation restore keyboard focus treatment
and normal control activation. Editable fields, dialogs, active menus and
modified shortcuts retain their own keys. Playback is requested only for the
user's Space press; key repeats do not issue repeated toggles on pointer controls.
The input observer is owned by the app lifetime, not restarted by track updates.

## Verification

Production UI compilation and all 157 existing unit tests pass. New browser
regressions cover pointer close/selection followed by Space, deliberate keyboard
activation, real-time repeated collapse, reversal retaining Animation identity,
scroll during exit, and deferred focus across selector sessions. Existing checks
cover the connected surface/shadow and opaque paint over artwork, intermediate
frames, stationary text, outgoing inertness, delayed navigation, Escape,
replacement, focus restoration, Reduced Motion and zero completed menu effects.
Browser engine counts and native delivery are recorded below after completion.

This repair does not change exported Sketch geometry, artwork brightness,
preferences, database schema, playback protocol, embedded models or OS work.

## Delivery

Desktop source `512d2a0` is mirrored to primary as `6f66190`. The locally signed
build is installed, SHA-256:
`789eba8d1d3ed8e874d5360e04ff41df457984eea9bc3961900cb00d40cc3d47`.
Strict signing verification passes. All eight protected runtime/model hashes
match the previous app. Release, previous app and immediate private latest-profile
checkpoint are retained in `releases/desktop-junk-menu-2026-10-26/`.

A fresh native Play-button check on paused Intermission preceded quit/install.
The new app launched paused at its saved 2.308479 seconds (displayed as 0:02).
Junk's full cover is visibly restored in the real library. The native selector
opens as an opaque connected surface over that cover, with its outer shadow and
up chevron, then closes through Escape with focus returned to its trigger. Space
from that returned pointer focus changes Play to Pause without reopening the menu.

The control tool rejected the subsequent pause twice because the user changed
the window during input. The next read showed Fancy That and then Noises playing;
no commands were sent after that track change. The user was asked whether to
leave current playback or restore paused Intermission. The disk playback snapshot
still matches the immediate pre-install checkpoint, but is not evidence of live
playback after this native test. The release diagnostics distinguish both states.

Six collection tables, all 34 favorites, 508 tracks, interface preferences and
integration configuration match the immediate checkpoint. Audio files remain
unchanged. Native screenshots are in `out/qa/os-foundation/junk-menu/`.
Ten focused Chromium and ten focused WebKit cases pass, together with the existing
157 unit cases. The WebKit window/scale/theme matrix passes its 48 combinations.
Reversals retain the existing Animation objects instead of allocating another
three effects; completed menus leave none. No native display-pacing/FPS claim is
made from these browser checks.
