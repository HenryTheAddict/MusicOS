# Heart centering and sidebar cleanup — 2026-10-22

Desktop source `dc686aa`, integrated with the OS branch as `9764bba`. This changes desktop layout and tests only.

## Changes

- Removed the inherited text baseline from the layered heart wrappers. The button, beat, feedback, empty/filled layers, and SVG now share the same center. Sizes, supplied artwork, theme ownership, active-song-only BPM pulses, favorite toggling, and Reduced Motion remain intact.
- The visible sidebar now has Home, Search, Albums, Artists, Songs, and More. Genres, Favorites, Playlists, Sparklists, and creation actions moved into the accessible More menu. Home’s collection filters and existing management flows remain available.
- Henry’s stuff has a persistent padded rounded sidebar box; up to two manual playlist drop shortcuts live inside it. The main personal-home heading also has a solid rounded surface. Spacing is consistent around the search slot and navigation; More is left-aligned.
- The connected section dropdown’s label and typography stay stationary. Its symmetric chevron folds vertically around a fixed center over 140 ms instead of rotating around a text-sized wrapper. Rapid reversals use CSS’s current state; Reduced Motion switches directly.

## Evidence

Before the fix, four newly added regression cases failed with SVG vertical offsets of **2.203125, 3, 4, and 4 CSS pixels** at 80/100/125/150%. After the fix, both heart layers report **zero horizontal and vertical offset** at every scale. Buttons also match the adjacent row action’s center within subpixel rounding.

115 UI unit tests and 32 browser tests passed. Coverage includes album/playlist/Songs BPM hearts, favorite toggling, offscreen and Reduced Motion behavior, main/mini player geometry in six light/dark source combinations, 1×/2× alpha, section label/chevron geometry and reversals, Home navigation and Resume queue preservation, model status polling, and collection Cancel preserving sequences. The new sidebar-box matrix covers 720×480 and 1280×800 at four scales in Light/blue, Dark/artwork, and extreme Dark/custom. Secondary creation remains reachable from More with keyboard-operable dialogs. Existing personal-home tests cover forty scale/size/theme combinations.

Logs, baseline failures, final regression results, and the fixture screenshot are retained under `out/qa/os-foundation/sidebar-heart/`. The initial failures are intentional before/after evidence, not unresolved tests.

## Signed candidate

Candidate: `releases/desktop-sidebar-heart-2026-10-22/MusicOS.app`. Ad-hoc signing and deep strict verification passed.

Executable SHA-256: `aac9d686a8e6987a915a65e01703609da24012a17c78ccf7f5c51c28da8976b2`

Unchanged embedded hashes:

- `musicos-mac`: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`
- ONNX runtime: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`

**Native installation pending.** The user was actively changing playback, Settings, collections, and navigation during final rollout. The pause/quit attempts did not complete, and the raise attempt reported that the user had changed the app; the main app remained running and playing. No bundle replacement or stale-profile restoration occurred. An asynchronous request asks the user to pause and leave the window untouched briefly. Before publishing, confirm paused/stopped state, quit, make a fresh private preference checkpoint, retain the app actually installed at that time, verify a staging bundle, then rename and check real rows/Home/dropdown. Update the release manifest after verification.

Subjective design acceptance, physical touchscreen behavior, Windows/Linux, full native resize/focus testing, and packaged frame intervals on a physical 120 Hz display remain separate gates. No guaranteed refresh rate is claimed.
