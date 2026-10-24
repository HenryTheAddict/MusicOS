# MusicOS for Vesktop

A local Figtree font and solid charcoal/cyan surfaces matched to the MusicOS palette visible on 2026-10-23. This is a fixed theme, not automatic album-color synchronization. The font uses the existing Figtree OFL license; installation embeds both font and license without a remote font request.

Run `node scripts/theme-vesktop.mjs` to preview its purpose, and add `--apply` to back up the existing Quick CSS before replacing it. Vesktop’s existing Quick CSS setting must be enabled. The installer does not change plugins, credentials, privacy settings or Rich Presence permissions. It prints the backup location for restoration. Copy that saved CSS back to the printed destination to restore it.

For MusicOS presence, keep Vesktop’s existing Rich Presence via arRPC enabled. It exposes the compatible local IPC endpoint; MusicOS still uses the existing Discord application ID and exact public artwork lookup. Upload the MusicOS application icon once as Rich Presence asset `musicos` for the fallback; no local album art is uploaded automatically.
