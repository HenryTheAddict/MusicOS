# musicOS iPhone companion

This is a dedicated Tauri iOS app, with the shared Figtree/Sketch presentation and a lightweight native backend. It does not embed the desktop audio models, visualizer, or OS runtime.

## Build

Install Xcode and the iOS Rust targets through rustup. From the repository root:

```
node ios/scripts/build.mjs --target aarch64-sim --debug --no-sign
MUSICOS_APPLE_TEAM=YOUR_TEAM node ios/scripts/build.mjs --target aarch64 --debug --export-method debugging
```

The build generates the Xcode project, exports the current app-icon master into opaque iOS icon sizes, and bundles the phone frontend. Generated projects, credentials, provisioning profiles, and signed products are excluded from version control. The signing team is supplied locally rather than hard-coded into source. Personal Team profiles expire after seven days.

## Companion behavior

Home, Search, and Library retain destination state. The player stays above navigation; its expanded view cycles Now playing → Up next → Lyrics. Native audio supplies lock-screen controls, seeking, output routing, interruption handling, queue identities, shuffle, and repeat. The portable decoder prepares formats unavailable to AVPlayer; originals remain byte-identical. Cached lyrics are read offline without initiating provider searches.

Computer offers camera QR scanning, pairing-link entry, Bonjour discovery, explicit Mac approval, preparation/download progress, cancellation, and resumed range downloads. The connection pins the Mac certificate. Pairing secrets remain in Keychain. Each downloaded file is hashed before publication. Library asset paths are relative on disk so an app update can relocate the iOS data container safely.

Favorites and complete manual playlists have a durable offline journal. Duplicate entries and ordering survive transfer. Operations are idempotent and revision-checked; conflicting playlist edits preserve both versions. Partly transferred collections cannot overwrite the Mac's full sequence. Older Mac builds can pair and copy audio without the newer mutation capability; their phone edits remain pending until a new approved pairing enables it.

Sparklists use the exact saved Mac sequence; generation remains on the Mac. The first companion does not silently upload audio, publish lyrics, or replace the Mac library. Deleting downloaded audio is separate from editing playlists.

## Checks

`node ios/scripts/simulator-check.mjs SIMULATOR_UDID` builds isolated original FLAC, Vorbis 256/320, and AAC fixtures, then checks native advancement, seeking, pause, duplicate queue identities, local lyrics, playlist journaling, and system media information. It refuses non-fixture libraries. The harness is compiled only in debug builds and runs only with an explicit environment flag.

Simulator results do not establish audible quality, Bluetooth/AirPlay behavior, camera scanning, physical background execution, or conflict UX acceptance. Record those separately on the installed phone. CarPlay and app-owned iCloud capabilities require their own signing and account checks; do not expose an enabled feature merely because its framework compiles.
