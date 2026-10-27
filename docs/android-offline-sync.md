# Android offline listening and Wi-Fi transfer

MusicOS Android is a native Android client. It keeps the supplied Sketch icons, Figtree, rounded solid blue surfaces, Home/Search/Library navigation, an acknowledged Play/Pause morph, and a player inside the app. The Mac remains the library source. Copying never starts or pauses Mac playback.

## Using it

1. Install the signed APK on the phone. Connect the phone and Mac to the same private Wi-Fi.
2. In Mac **Devices**, choose available library, Favorites, or a collection, then **Start Wi-Fi sharing**. Preparation hashes the selected originals and prepares small SDR artwork copies.
3. Choose **Create pairing link**. Scan the QR with the phone camera or paste the link into Android **Devices**. It expires after two minutes and works once. Approve the matching phone and six-character request on the Mac.
4. On Android choose **Copy music**. Allow local-network access when Android asks; notifications show transfer progress and playback controls. Transfers wait for unmetered networking and adequate storage. Keep the Mac app open until copying completes.
5. Disconnect from Wi-Fi: verified downloaded songs, covers and collection sequences remain available. Headset, Bluetooth, notification and lock-screen actions control the phone's playback session.

Stop sharing to close the listener and discovery. Sharing is always off on Mac startup. Forgetting a phone revokes future access immediately and retains its existing offline copies. New copies retain phone favorites and previously downloaded music; this is additive, one-way transfer, not two-way metadata editing or remote deletion. There is no cloud account, mobile Discord RPC, automatic local-art upload, or transcoding. Unsupported device codecs can be reported by Android playback without changing originals. FLAC, MP3, AAC/M4A, Ogg/Opus, WAV and AIFF are offered as original files; actual decoding depends on the device.

## Audio and Android integration

A single Media3 `MediaLibraryService` owns ExoPlayer, audio focus, becoming-noisy handling, the media notification, persistent queue/position and trusted external controllers. Restored playback stays paused. Standard media-library roots expose albums, artists, songs, collections and favorites, with paged browsing, voice/text search, duplicate queue identities, ratings, seek, repeat and shuffle. The app exposes its audio session to the system equalizer and opens the Android output switcher where available. Settings can open notification preferences. Download work has a separate cancellable progress notification. Background playback survives Activity recreation; visible position sampling stops when paused or hidden.

Android Auto, real Bluetooth/headset routes, OEM equalizers, physical touch and battery-management behavior require real-device acceptance. Declaring MediaLibraryService and the automotive descriptor enables the integration; it does not establish certification or real-car verification. [Media3 background playback](https://developer.android.com/media/media3/session/background-playback), [serving a media library](https://developer.android.com/media/media3/session/serve-content), [Android local-network permission](https://developer.android.com/privacy-and-security/local-network-permission).

## Transfer contract, version 1

The opt-in Mac listener uses pinned TLS and `_musicos-sync._tcp.local.` discovery. Discovery contains only the server identifier, public certificate fingerprint and protocol version. A pairing link contains a random 256-bit one-time ticket and a certificate pin. Android verifies that pin before sending its independently generated 256-bit client credential. The Mac retains only a credential hash; Android seals its pairing record with Android Keystore AES-GCM. Tokens stay in Authorization headers, never media URLs, manifests, caches or logs. Pair requests are rate-limited and require Mac approval. The listener rejects nonlocal source addresses. Credentials, private signing material and pairing links must not be committed or included in diagnostics.

- `POST /v1/pair`: ticket, client_id, client_name, client_token. Creates a pending request.
- `GET /v1/pair/status`: pending/approved using the phone credential.
- `GET /v1/manifest`: selected tracks with metadata, byte length, SHA-256, artwork hash and collections with ordered track IDs. No local filesystem paths. Server identity and revision are checked.
- `GET /v1/tracks/{id}` and `/v1/artwork/{hash}`: allowlisted immutable snapshot files, bounded single ranges, ETag and Content-Range. Revocation/stop terminates streams. Changed originals require a refreshed selection.
- `POST /v1/sync-complete`: revision, track_count, bytes; records the phone's receipt only for the matching snapshot.

The phone downloads to partial files, validates resumed ranges and final byte count/SHA-256, then publishes checksum-named files. A SQLite transaction adds metadata only after the complete selection is verified. Cancellation keeps completed objects and resumable partials. Corrupt partials are discarded. Reconnection discovers only the matching server identity and pin. Revoked credentials require pairing again. Local backups are disabled for Android's credential-bearing app state. No Mac database migration, embedded OS protocol or installed model/embedding changes are involved.

## Building

Use JDK 21, an Android SDK with API 37, and the checked-in Gradle wrapper. Put `sdk.dir` in ignored `android/local.properties`, or set `ANDROID_HOME`. Do not commit local SDK paths or keystores.

```
android/gradlew -p android :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
android/gradlew -p android :app:connectedDebugAndroidTest
```

The app supports Android 8/API 26 and later, targets Android 17/API 37, and requests local-network permission on API 37. Testing uses an isolated emulator, synthetic PCM audio and temporary libraries. Native Kotlin audio is intentionally separate from the desktop Tauri backend, which depends on macOS/desktop services.

For a signed release, set `MUSICOS_ANDROID_SIGNING` to a private properties file containing `storeFile`, `storePassword`, `keyAlias`, and `keyPassword`, then run `:app:assembleRelease`. Preserve that keystore for future updates; losing it prevents installing updates over existing copies.
