# Wi-Fi startup, Vorbis copies and phone presentation — 2026-10-28

## Flow findings and resulting behavior

1. **Choose music and format — repaired.** The supplied collection screenshot showed three competing creation controls; creation now lives in one connected Create menu. Devices groups music selection, Original / Vorbis 256 / Vorbis 320, an approximate copy-size estimate and one primary action. Source tracks are never overwritten. The existing All available music choice covers the available library; Favorites and saved collections remain available.
2. **Start and pair — repaired.** The supplied Devices screenshot showed sharing off while a long preparation operation blocked pairing. The opt-in TLS listener now opens first. Preparation runs separately with song counts, current title, cancel/stop and errors. Pairing is available during preparation. The phone waits and shows progress in the active transfer instead of scheduling repeated WorkManager retries for a 503. Sharing still defaults off at app launch.
3. **Copy and listen on the phone — emulator verified.** Home has artwork-led collections and a compact resume row; Library has a two-column album layout and Albums/Artists/Songs with other views in More. The player measures remaining artwork space, keeps seek/transport together and expands volume in one row. Phone header contrast and singular song counts were corrected. Custom bottom-tab feedback retains the existing finite jelly motion without the default sparkle overlay. Search query, collection identities and queue duplicates remain intact.

Supplied screenshots and current-run evidence are in `out/sync-vorbis-2026-10-28/`: `01-collections-before.png`, `02-devices-before.png`, `devices-480.png`, `devices-720.png`, `devices-1280.png`, `devices-preparing.png`, and Android screenshots. The browser captures use explicit fixtures, not the user's library. The mobile fixtures use existing project artwork and generated silent audio.

## Preparation and audio details

- Persistent private preparation index reuses unchanged source hashes and SDR thumbnails. Canonical path, size, modification time, and Unix device/inode/change time invalidate entries. Cached output stamps are checked too; source edits during preparation are rejected. Album membership is indexed once instead of searching every album per song.
- Separate Vorbis objects use FFmpeg/libvorbis when available, otherwise FFmpeg decoding piped to installed Xiph oggenc. The experimental FFmpeg native Vorbis encoder is not used: validation found that its headers did not advertise the requested target. Missing supported encoders disable the two Vorbis options with an explanation.
- Vorbis profiles use 48 kHz stereo with 256000/320000 nominal bitrate. Actual bitrate and size vary with the audio, as documented by [FFmpeg](https://ffmpeg.org/ffmpeg-codecs.html#libvorbis) and Xiph oggenc. The phone's manifest retains metadata, cover identities and collection order. Credentials never enter the preparation index.
- Preparation has one owner. Rapid restart/stop invalidates old work, checks cancellation during hashing and encoding, kills encoder/decoder children on cancellation, and prevents stale publication. Completed conversions are checkpointed. Existing sharing credentials and peers are retained.
- Approved phones can poll only minimal preparation progress. Manifests return 503 during preparation and 409 on preparation failure. Optional `If-Match` checks reject a different file version with 412; byte count/hash verification remains required before publishing downloads. Original clients remain compatible.
- Changing profiles updates each song's active file. Old downloaded objects are retained; this change does not silently delete previous phone copies. No database migration, playback protocol, model, embedding or OS changes.

## Verification

- Rust suite: 170 passed, one explicit emulator-server fixture ignored. Final targeted Wi-Fi suite: ten passed, one ignored. Tests cover credential checks, TLS pairing during preparation, bounded ranges, stop/restart races, wrong-version rejection, source-change invalidation, cancellation and reuse.
- Both Vorbis profiles: validated Ogg identification headers at 256000 and 320000, successful decoding, distinct hashes, reused output modification times and unchanged original bytes.
- Debug-build preparation measurement, eight generated files totaling 64 MiB: cold hashing 2462.9 ms; reused stamps/hashes 0.38 ms. This isolated cache measurement is not a throughput prediction for the user's library or a native FPS claim. The listener-before-preparation test completes within its two-second bound.
- UI unit suite: 161 passed. Seven WebKit Home/Devices tests passed; two Chromium copy-format/layout tests passed. Devices was checked at 480/720/1280 widths and 80/100/125/150% scales, including preparation/cancel/pairing, full heading visibility, 44 px format targets, no horizontal overflow and no playback commands. Existing Home matrix covered blue, Light/Dark artwork and extreme custom colors.
- Android unit tests, lint and signed release build passed. Three Android 17 emulator integration tests passed: Home/Library navigation and retained Search, Light/Dark/short/tall player with visible expandable volume and duplicate queue entries, and both Vorbis profiles playing offline through the existing Media3 session with muted synthetic fixtures. No Mac playback was used for tests.

## Delivery gate

The isolated desktop branch is `codex/desktop-heart-preservation`; OS work remains in the primary repository. Delivery requires a fresh paused/stopped check, a latest-profile/library checkpoint, previous-app preservation, a verified native signature and matching eight protected embedded sidecar/runtime/model hashes. Signed build hashes and post-load state are recorded in `/Users/henry/Developer/MusicOS/releases/desktop-wifi-vorbis-2026-10-28/release.json`.

Physical Android Wi-Fi transfer speed, OEM battery restrictions, real touch/font scaling and hardware audio routes remain physical-device gates. Emulator decoding and TLS fixture tests do not establish those outcomes.

## Prepared delivery status

Source commit `9518c8c`. Signed desktop executable SHA-256: `daf2b88506fe06cccd576be2f8bc6bb854c8a752a5aac190f649adf317bd2210`. Signed Android APK SHA-256: `8585dd5a60ac465a0d0701400414dc14ab3de3facebea4fddfd191626d816c00`. All eight protected embedded hashes match the currently installed desktop app.

The connected Samsung's personal-profile musicOS app was updated in place after observing stopped playback and no active copy service. Its previous APK was preserved; old/new signing certificates match, installation used `-r --user 0`, and pulling the installed APK confirmed the exact signed-release hash. The private secondary phone profile was not accessed. Physical phone UI/audio/Wi-Fi operation remains unverified; no test fixtures were installed in its library.

Desktop replacement is pending the user's pause: the final native check showed ongoing playback. No Mac playback command, app replacement, profile restore or library rollback was performed. The new app, APK and setup notes are archived under `/Users/henry/Developer/MusicOS/releases/desktop-wifi-vorbis-2026-10-28/`; `release.json` currently records desktop `installed: false`. The primary OS repository remains clean. The temporary emulator was stopped after its tests.
