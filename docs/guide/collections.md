# Playlists, Sparklists, and Smart playlists

[Guide home](README.md)

Click your personal collections heading in the sidebar, or use **Settings → Library → Manage collections**. Search by name or description and sort by name or recent changes.

## Three collection types

- **Playlists** keep songs in your chosen order. Create one or import M3U; open it to edit songs, details, or artwork and export it.
- **Sparklists** are experimental audio-based mixes. Creation uses Choose a vibe → Preview → Name & save, with model setup inline when needed. Previewing does not start playback. Saved songs stay fixed until you explicitly choose Refresh mix.
- **Smart playlists** follow rules. Edit all/any matching, individual rules, and an optional song limit. Existing rules are preserved when you rename a collection. These are the older rule collections, now named clearly rather than presented as audio generation.

Opening, renaming, changing artwork, or editing a collection does not rewrite an active playback queue. Delete requires a confirmation in the collection interface. Source music is never deleted by collection management.

Sparklist model installation and consent remain in **Settings → Audio → Sparklist**. The manager and creation flow link there. Embedding coverage says how many songs can participate; it is not a recommendation-quality score or BPM/key accuracy measurement.

## Create or add songs

**Create playlist** opens a compact sheet. Enter a name, choose square cover artwork in its own picker, and optionally choose several songs before creating it. **Import M3U** is a secondary action.

Right-click a song, or open its overflow button, for **Play next**, **Add to queue**, and **Add to playlist…**. The last action opens a searchable list of manual playlists. Audio Sparklists and Smart playlists manage their own sequences and do not accept manual membership edits.

## Edit a manual playlist

Click a song row to select it, Command-click to toggle more songs, or Shift-click to select a range. Command-A selects visible songs. Drag selected rows to reorder them; Delete removes them from this playlist. Playback buttons stay separate from selection. **Edit details** opens a compact sheet for name, description and artwork.

**Add Music** opens Songs/Albums browsing with search. Plus adds immediately and becomes Added; Done closes the browser. Album and selected-song context actions, and dragging selected library rows to a manual playlist in the sidebar, use the same membership operation.

**Suggested additions** offers up to ten local recommendations based on the playlist members’ cached audio embeddings. It excludes members and verified duplicate recordings and applies artist/album diversity penalties. Add songs explicitly with plus; suggestions never change an active queue. Empty playlists offer a starting-song selector. Missing experimental-model consent/setup links directly to setup; limited coverage produces fewer suggestions. This does not replace the pinned model or claim listening acceptance.

Selection and membership editing apply to manual playlists. Sparklists keep explicit generation/refresh, and Smart playlists retain complete rule definitions.
