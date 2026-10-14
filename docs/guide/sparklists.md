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

New generation uses bounded artist/album diversity penalties so repeated contributors cannot progressively outweigh audio similarity. This ranking update does not replace the experimental model or establish listening quality.

Listen to preview, Regenerate for another ranking, or use Less like this for explicit negative feedback. Ordinary skips and unliking do not train negative preferences. Give the preview a name and save it. Saved sequences stay unchanged until explicit Refresh. Convert to a manual playlist when you want to edit the sequence yourself.

Preview includes deterministic sound-derived abstract artwork in the style of the Sketch auto-covers. Cached embeddings and mood similarity determine its color field; available energy and confident tempo are included in its versioned parameters. Regenerate artwork changes only the cover. Manual cover choices and album collages remain available. Saving preserves the generated cover across restarts.

After installation and analysis, generation runs locally and can work offline. Provider artwork lookups are separate. Model removal disables new generation; it does not delete saved collections. Changes to collections do not rewrite an already-playing queue.

Manage saved mixes in the **Sparklists** tab of [Collections](collections.md). Smart playlists are separate rule-based collections.

## Names and artwork

Name & save suggests editable labels from your chosen mood, the starting song (for example, **November-esque**), and the cover's color family. A suggestion is a creative label, not an audio-analysis verdict. Your typed name stays intact when you return to Preview.

New generated covers offer additional soft fields, blobs, ribbons, and rounded star arrangements. Regenerate artwork changes only the artwork. Sound colors use the cached audio-derived choice; Mood colors use your selected mood; Seed cover colors use the starting song's canonical cover palette. These color choices do not change recommendation ranking or regenerate the song list.

Saved version-one artwork keeps its existing rendering. New versions and explicit artwork changes are cached deterministically and survive restart with the saved mix.
