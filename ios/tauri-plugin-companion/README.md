# musicOS companion native services

Private Tauri plugin for the iPhone companion. The request bridge owns native audio, system media controls, pinned Wi-Fi transfers, Keychain pairing, QR scanning, discovery, and the durable change journal. Audio stays native; no PCM is sent through JavaScript.

`request`, `register_listener`, and `remove_listener` are the only exported commands. Permissions are declared under `permissions/`. Simulator-only native checks require a debug build, an explicit environment flag, and a fixture-only library.
