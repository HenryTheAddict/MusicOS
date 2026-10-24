# MusicOS for Vesktop

A local Figtree font and MusicOS blue with mode-aware solid surfaces, requested on 2026-10-23. Dark mode uses the desktop’s computed blue background `#1e2631`, raised surface `#28313f`, and contrast-adjusted accent `#64a1ec`; Light mode uses the classic pale-blue palette. It follows the client’s existing Light/Dark selection, not automatic album-color synchronization. The font uses the existing Figtree OFL license; installation embeds both font and license without a remote font request.

Run `node scripts/theme-vesktop.mjs` to preview its purpose, and add `--apply` to back up the existing Quick CSS before replacing it. Vesktop’s existing Quick CSS setting must be enabled. The installer does not change plugins, credentials, privacy settings or Rich Presence permissions. It prints the backup location for restoration. Copy that saved CSS back to the printed destination to restore it.

Reload Vesktop once after replacing the file if its CSS watcher does not pick up the atomic replacement. Dark blue surfaces use pale ink, and Light mode uses dark ink; profile panels and the message composer have explicit surface overrides so they do not retain dark defaults beneath that ink. User avatars, banners and message images retain their own colors.

For MusicOS presence, keep Vesktop’s existing Rich Presence via arRPC enabled. It exposes the compatible local IPC endpoint; MusicOS still uses the existing Discord application ID and exact public artwork lookup. Upload the MusicOS application icon once as Rich Presence asset `musicos` for the fallback; no local album art is uploaded automatically.
