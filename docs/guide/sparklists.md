# Audio Sparklist beta

[Guide home](README.md)

Sparklist ranks your local songs using cached audio embeddings. Metadata filtering is not a substitute for this analysis. The replacement model remains experimental pending blind listening evaluation.

## Set up

1. Open **Create Sparklist**. If needed, enable Experimental audio Sparklist beta in its inline setup. Settings → Audio → Sparklist also manages the installation.
2. Install the staged, checksum-verified replacement model. The creation screen supports the staged candidate or a selected staged folder without losing your seed. Public model download is not available yet.
3. Wait for installation to finish. You can cancel, retry a failed installation, or remove the model here. Progress and actual model size are shown in the app.
4. Open **Quality & analysis** and analyze your library, or analyze the selected starting song directly from creation.

The supported candidate is LAION `larger_clap_general`, with pinned manifest, preprocessing, attribution, and ONNX runtime checks. An arbitrary or earlier rejected model cannot be substituted.

## Make and keep a mix

Creation has three stages: **Choose a vibe → Preview → Name & save**. Choose a starting song and/or mood, then Preview Sparklist. Choose for me selects a song with cached audio embeddings and prepares its preview without starting playback. At least two songs with cached embeddings are needed by creation; coverage limits the mix and does not establish recommendation quality.

Choose **Length** beneath the vibe controls: 10–100 songs, initially 25. The slider also works with the arrow keys and Home/End. Changing length keeps your starting song and feedback but clears the old preview; choose Preview to generate again. The preview shows its actual count and total listening time. If fewer analyzed songs are available after feedback exclusions, it returns those songs once and explains the shortfall.

In a saved Sparklist, **Edit details → Length → Save details** changes the requested size for its next **Refresh mix**. It does not change the saved songs immediately or alter an existing playback queue. Older saved mixes default to a requested length of 25 and keep their current sequences.

New generation uses bounded artist/album diversity penalties so repeated contributors cannot progressively outweigh audio similarity. This ranking update does not replace the experimental model or establish listening quality.

Listen to preview, Regenerate for another ranking, or use Less like this for explicit negative feedback. Ordinary skips and unliking do not train negative preferences. Give the preview a name and save it. Saved sequences stay unchanged until explicit Refresh. Convert to a manual playlist when you want to edit the sequence yourself.

Automatic cover choices use the supplied Sketch Sparklist covers, selected deterministically from the preview’s audio-derived artwork parameters. Regenerate artwork changes only the cover. Existing generated artwork and manual choices remain available and saved covers stay unchanged until explicitly changed.

After installation and analysis, generation runs locally and can work offline. Provider artwork lookups are separate. Model removal disables new generation; it does not delete saved collections. Changes to collections do not rewrite an already-playing queue.

Manage saved mixes in the **Sparklists** tab of [Collections](collections.md). Smart playlists are separate rule-based collections.

## Names and artwork

Name & save starts with an empty name. When a starting song is selected, its cleaned title offers one optional suggestion, such as **November-esque**. Generic mood/color slogans are no longer inserted. Your typed name stays intact when you return to Preview.

**Select song** opens a separate song picker with artwork, artist credits, durations, and an optional filter. Choosing a song closes the picker and keeps the creation stage in place. Escape closes the picker first and restores focus; it does not close Sparklist creation. **Change song** reopens the picker with the current selection. Ready creation does not show the full embedding-coverage count; incomplete preview coverage remains available in a disclosure. Listen to preview explicitly starts its sequence; the same control pauses and resumes it. Playback errors appear within creation and can be retried. Saved versioned artwork keeps its existing rendering.
