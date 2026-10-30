# Configuring musicOS

`product.config.json` is the shared build-time configuration for desktop UI, the native desktop defaults, and Android. It contains no credentials or user profile data.

| Setting | What it controls |
| --- | --- |
| `name`, `version` | Product display name and About/welcome labels. Android label/version are generated from this file. |
| `repositoryUrl` | Public project link used in provider identification. Set this to your project’s actual URL when forking. |
| `defaults.libraryLabel` | Home/sidebar fallback when someone leaves their name blank. |
| `defaults.interfacePercent` | First-run desktop scale: 80, 100, 125 or 150. |
| `defaults.miniPlayerDensity` | First-run Standard, Compact or Wide preset. |
| `defaults.appearance` | First-run system/light/dark mode, blue/artwork/custom source, accent, saturation and surface tint. Android supports blue/artwork sources. |
| `storage` | Names for a fresh desktop profile’s managed media, library database and sync directory. These must be single relative names, not personal paths. |
| `onboarding` | Flow version and welcome titles on desktop and phone. |

After changing product identity/version, run `npm run product:sync`, then `npm run product:check`. Sync updates the desktop package name/version, HTML title and Tauri crate version; Android reads the same file at build time. `build:ui` validates configuration automatically. Runtime templates use the shared product name, though explanatory sentences can still mention musicOS; this is not a translation or white-label framework.

Keep existing Tauri/Android application IDs and legacy profile directory identities stable for updates. Changing them creates a separate app/profile and breaks update continuity. The sync script intentionally leaves those identifiers alone. Versions for unrelated workspace/OS crates are independent.

Defaults apply to new users. They do not reset saved names, palettes, scale, density, music folders, playback, collections, integrations or credentials. Existing absolute storage paths remain unchanged. Fresh profiles always use app-data storage; launching from a folder that happens to contain `music/` no longer adopts it as a library.

For an isolated development profile, existing overrides are available: `MUSICOS_APP_DATA`, `MUSICOS_DESKTOP_CONFIG`, `MUSICOS_MEDIA_DB`, `MUSICOS_MEDIA_DIR`, and `MUSICOS_SYNC_DIR`. Use a separate test directory rather than editing someone’s profile. No personal home directory is compiled into production startup settings. Names and absolute local paths in fixtures/tests are test data.

Desktop onboarding uses an optional typed `onboarding_version` preference. Missing markers on existing valid preference files are treated as already set up. A genuinely new empty profile starts at zero; completing setup saves its current version, name and appearance through the normal preference writer. Saving must succeed before the flow closes. Existing users can open welcome setup from Settings → General. Escape closes it for the current session without changing preferences. It never starts playback, sharing, model installation or a scan by itself. Folder selection adds references only after the user chooses a folder; starting empty is supported.

Android uses the same version in its existing appearance preferences. Previously configured, paired or populated phones keep their setup. New phones get welcome, appearance and pairing guidance; exploring first is supported. Camera/network/notification permissions are requested only when the corresponding user action needs them. Pairing does not silently start a library transfer.

Trackpad back navigation is in `ui-desktop/src/useBackSwipe.ts`: `backSwipeThreshold` is the deliberate horizontal distance (110 px), with a short 130 ms end-of-gesture delay. Browser wheel input does not expose a finger count; this supports the standard two-finger trackpad horizontal gesture. Shift-wheel, pinch/zoom, vertical scrolling, editors, sliders, menus, queue panels and horizontally scrolling shelves retain their own behavior. A gesture navigates once after release and clears its temporary cue; reduced motion omits spatial movement. It reuses navigation history, scroll restoration and artwork return motion.
