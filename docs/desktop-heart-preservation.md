# Desktop heart and motion preservation

This desktop patch is isolated from unfinished OS changes on `codex/desktop-heart-preservation`, based on checkpoint `26d3356`.

- Song-row and mini-player BPM pulses remain intact. The main Now Playing bar now exposes the same favorite heart while browsing Albums or Artists.
- Pulse speed uses valid cached analysis, affects only the playing song, stops while paused/hidden, and respects Reduced Motion.
- Tempo display now fetches only track IDs and BPM. The current profile's raw full-analysis JSON totals 55,373,845 bytes; the corresponding tempo-only JSON totals 15,252 bytes. This is a payload comparison, not a measured frame-rate improvement.
- Temporary request failures retain valid tempos and get bounded retries. Analysis completion and newly playing uncached songs refresh the tempo display.
- The original embedded preview executable and ONNX runtime are preserved byte-for-byte.

Verification: 139 native tests, 105 UI unit tests, 3 heart behavior checks, and 15 album/artist/navigation motion checks pass. Compact/large header geometry, pause/resume, favorites, and Reduced Motion are covered. Human motion acceptance remains separate. Windows/Linux were not tested for this patch.
