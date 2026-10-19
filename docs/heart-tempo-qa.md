# Analyzed-tempo song hearts — 2026-10-17

Song-row hearts in albums, manual playlists, Sparklists, Smart playlists, Favorites, and Songs share the existing TrackRows control. Only the currently playing song pulses, using its cached analyzed BPM, with one gentle 9% scale pulse per beat. Pausing stops it; the other rows remain still. Unknown, nonfinite, or out-of-range BPM stays static. This is a tempo visualization, not audible beat-phase synchronization; playback speed does not change the analyzed value.

One library-level read uses the existing analysis status contract after bootstrap. Analysis controls, rescan completion, and library identity changes refresh the map; a running job is checked every five seconds. Favorite changes do not trigger another analysis read. No analysis is started by the effect.

A shared IntersectionObserver enables animation only for visible hearts. Hidden documents pause it. Reduced Motion disables it. The icon layers, favorite action, focus treatment, and row geometry remain unchanged. Favorite feedback animates a separate element so it cannot cancel the recurring pulse. No per-frame JavaScript, playback/DSP changes, file writes, or collection changes are introduced.

UI build and all 86 unit tests passed. Two browser tests passed, then were updated and passed again for the user's playing-song-only correction: per-song periods in album/playlist/Songs views, switching the active heart, pause/resume, favorite interaction during pulsing, missing BPM, refreshed/invalid BPM, off-screen suspension/restoration, and live Reduced Motion changes. The initial revised test incorrectly clicked a hover-revealed Play control without hovering; the test was corrected to use the actual row-hover interaction.

Final macOS executable SHA-256: `4daee5674c792f7918373e84b40bca983bef5682db03e189066f52c1070c2619`. Ad-hoc signing and deep strict signature verification passed. The embedded playback (`6b1fa66d…`) and ONNX (`ff52828c…`) runtime hashes are unchanged.

The corrected package was loaded with playback paused. Native IGOR album inspection retained the song controls and layout, with I THINK paused at 22.4 seconds, 507 songs, and the user's current 16 favorites. No favorite, collection, or queue edits were made by this native pass. A transient macOS capture error recovered on reconnect. Active pulse timing, active-song switching, and pause/resume are verified by browser animation checks; human animation preference and Windows/Linux remain unverified.
