# Headphone output recovery and menu contours — 2026-10-27

The desktop player previously opened one output stream at startup and never rebuilt it. A route change or stopped callback could therefore leave Play toggling the mixer behind a stale output. Startup failure also discarded the mixer, preventing later recovery.

The new output owner reconnects to the selected system output when the Core Audio device ID or nominal sample rate changes, or after two seconds without source callbacks. It checks on the existing playback worker (at most four times a second while active; normally once a second while paused), including immediately on Play. Failed opens retry at most once a second. It does not enumerate alternate devices as a fallback while headphones negotiate.

The single mixer is moved by value from the disposed stream back to the worker, then into the replacement stream. Decoder workers, buffered frames, source clocks, queue entry identities, selection, pins, shuffle, volume, EQ/reverb state, and transport intent are retained. No decoder reopen, seek, or queue restart is involved. Healthy prepared decoders do not expire while the output is unavailable. Pause remains paused; a pending resume survives an unavailable output. The callback heartbeat adds one relaxed atomic update per 2,048 samples with no per-sample locking or allocation.

Read-only native inspection uses Apple's [default-output device property](https://developer.apple.com/documentation/coreaudio/kaudiohardwarepropertydefaultoutputdevice) and [nominal sample-rate property](https://developer.apple.com/documentation/coreaudio/kaudiodevicepropertynominalsamplerate). It does not modify macOS output preferences.

The menu-corner defect was a legacy connected-menu override: open triggers inherited a parent with square corners and gained an extra solid fill. Removing those overrides keeps each control's own rounded contour and preserves the transparent category action overlay. Existing panel reveal/retraction ownership is unchanged.

Validation:

- 90 desktop native tests passed, including real muted output disposal/reopening, same-mixer reclamation, callback watchdog and retry timing, missing-route retention, duplicate queue identities, non-rewinding playhead, paused recovery, and resume after reconnection.
- 166 Tauri tests passed (one explicit Android fixture ignored); 161 UI unit tests passed; UI build passed.
- Chromium menu/header regressions passed. Four focused WebKit checks passed, including corner-pixel comparison, contour/fill retention, connected selector collapse, reversal/replacement, and final closed paint.
- Physical wired/Bluetooth switching and audible routing remain user acceptance checks. Automated native testing reopened the actual local output at zero application volume using synthetic silence; it did not change the selected output or operate the user's playback.

Delivery stays on the isolated desktop branch, with a fresh paused/stopped gate, latest profile checkpoint, previous app, signed executable hash, and protected runtime/model hashes recorded in the release archive. No Android, database, playback-protocol, model, embedding, or OS-source changes are required.
