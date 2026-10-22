# Saved Sparklist recommendations and mini-player queue motion

Delivered 2026-10-21 from isolated desktop commit `9a99dea`, integrated with OS work as `c3847e7`. The integration retains the connected-device command and uses the extracted core database adapter; embedded UI/runtime code was not rebuilt.

## Behavior

- Henry’s stuff shows one compact **For your mood** shortcut. **More** and empty Command-K show up to three ranked saved audio Sparklists. Text search still searches every saved collection; recommended entries are not duplicated in the empty search list.
- Local ranking weights the current song, then twelve recently played songs with decreasing influence. With neither, it uses up to 32 favorites, ordered by play count. It compares normalized cached audio centroids when the existing beta consent and verified model permit that; artist/genre overlap is the fallback. Small membership-overlap penalties diversify near ties. Empty/unavailable/manual collections are excluded. With no listening signal or useful match, no recommendation is invented.
- This is a listening-based estimate, not a mood diagnosis. Reasons identify current-song membership, audio similarity, or listening/favorite context. It opens existing mixes; no generation, automatic playback, queue replacement, new history store, or membership changes.
- The two entry points share one display request. Track changes, collection membership, library updates, model invalidation, and visibility refresh it. Playback-position updates do not. There is no timer or hidden polling.
- Mini-player content dissolves out for 75 ms before its layout changes, then fades in for 105 ms. Bottom controls remain fixed. Interrupted transitions resume from current opacity, stale completions are cancelled, and Reduced Motion switches directly. Queue DOM and scroll are retained; there are no duplicate image layers or per-frame layout measurements.

## Verification

- 112 UI unit tests and 144 Tauri unit tests passed, including three new ranking tests. The extracted-core integration passes those three tests separately.
- 24 distinct browser regression cases passed: the 22-case consistency, follow-up, and recommendation run plus two Settings/queue persistence cases. The four new recommendation/motion cases were rerun on final source and passed. Checks include all four actual interface preferences at 720×480 and 1280×800, six theme combinations for existing mini geometry, 1×/2× rounded transparency, retained footer geometry, rapid reversal, Reduced Motion, focus, shared suggestions, unchanged saved sequences, and no idle recommendation requests.
- Native signed build: For your mood shows real saved mixes in sidebar and Command-K. The mini-player opens/closes Up next, keeps the window size, shows 22 upcoming songs, and returns to the main window. Intro remains paused at 151.4 seconds. 507 songs, 21 favorites, ten Sparklists and the manual playlist remain visible. The previously browsed Severance album was restored after restart.
- Native captures: `out/qa/os-foundation/mood-recommendations/mood-recommendations-native.png` and `mood-mini-native.png`. Logs and release manifest are alongside them.
- No real collection, metadata, source, music-file, model, or queue mutation was used for QA. Disposable browser fixtures cover collection actions.

## Package and recovery

Ad-hoc signature passes deep strict verification. The installed executable SHA-256 is:

`aa50c019d8c3f1fdf87fd30f82d9f1df8986525dafc2be756eb8a18b58f4da8e`

Unchanged embedded hashes:

- `musicos-mac`: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

New/previous app bundles are retained under `releases/desktop-mood-recommendations-2026-10-21/`. Latest private preferences were checkpointed under application-data Development/checkpoints before replacement. No library migration was required. Publication used a verified staging bundle and rename while the main app was stopped; launch preserved paused playback.

Subjective recommendation and motion acceptance remain human checks. Physical touchscreen, full native resizing/focus acceptance, Windows/Linux, and measured packaged frame intervals on a physical 120 Hz display remain separate gates. No guaranteed frame rate is claimed.
