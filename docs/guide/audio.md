# Audio, quality, and DJmode

[Guide home](README.md)

## Listening controls

Click the **waveform/quality icon** for quick Audio controls: actual-track specifications, volume, EQ, pitch, speed, optional reverb and Reset. The **queue icon** opens Up next separately, including the DJmode switch. Choose Sidebar or Floating panel for Up next in **Settings → Audio → Playback**. Narrow windows use floating presentation automatically. Escape or an outside click closes floating menus.

EQ defaults to Bass, Mid and Treble. Settings → Audio → Playback offers three or eight bands; each keeps its own gains. Speed ranges from 0.5× to 2×, and pitch from −12 to +12 semitones. Link pitch & speed defaults on for record-player behavior. Unlink to adjust speed without changing the current pitch. Reverb defaults off; its compact square pad adds a damped stereo room. Move the circle left/right for room amount and up/down for shallow/deep decay. Arrow keys adjust either axis; Shift takes larger steps. Depth defaults to the original room and persists independently of the room amount. Higher depth extends the tail, with compensated room excitation to retain headroom. The direct recording stays at unity gain: enabling the room adds ambience rather than turning down the song. A subtle stereo spread decorrelates the wet tail, including on mono recordings; the dry signal stays unchanged and the wet mono sum is preserved before limiting. The volume rail has no visible knob and remains draggable and keyboard accessible. Speed/pitch changes update the running decoder worker rather than restart the song, with a short smoothing ramp. Audio distinguishes requested settings from mixer acknowledgement. If controls remain unconfirmed, it shows a local error and Retry audio controls. These preferences persist globally. Reset returns EQ, pitch, speed and volume to their defaults.

Drag songs into Up next to insert them into Up next. Song context menus also offer Play next and Add to queue. Repeated songs are separate queue entries. Manual moves pin their positions for the session. Queue edits do not modify the source collection.

Lossless codecs are decoded without a lossy encoding step. Normal speed, zero pitch, flat EQ and reverb off use dry PCM bypass. Speed, pitch, EQ and reverb deliberately alter samples; processed playback is not bit-perfect. Output still follows the desktop mixer’s stereo 48 kHz format and safety limiter.

## Quality and analysis

Hover or keyboard-focus the quality icon for actual-track specifications. Click it for quick Audio controls. Analysis management remains in **Settings → Audio → Quality & analysis**. Lossless/lossy comes from the codec, not the filename: ALAC inside M4A is lossless; AAC and E-AC-3 are lossy. Albums may have mixed or unavailable quality.

M4A playback, seeking, DJ preparation, analysis, and Sparklist preprocessing share native decoded audio on macOS. A supported extension does not guarantee a protected, damaged, or unsupported stream will decode. Failures need repair or a playable source copy.

Use Rescan audio for specifications and analysis. BPM, key, beat, phrase, and energy estimates remain estimates. Low-confidence analysis uses the safe transition fallback rather than claiming musical alignment.

## DJmode

DJmode itself is On or Off. Its separate settings select Smart transition or Full song, Subtle or Creative, and Preserve energy. Smart uses supported musical boundaries; Full song keeps the outgoing ending. Accepted transition schedules appear on the playback rail; canceled schedules disappear.

Smart transition compares measured beat timestamps across each candidate overlap rather than assuming a rounded BPM describes every beat. It requires supported structural boundaries, checks local tempo stability, and rejects predicted alignment drift above 50 ms. Candidates also account for confidence, local density/energy, harmonic compatibility, skipped material, and protected repeated sections. Half/double-tempo metadata alone cannot force a match; measured beat evidence is required. Preparation chooses a future cue, leaving time to decode the incoming deck. Weak evidence still produces an unwarped ending blend.

Structural analysis refreshes incrementally with upcoming songs prioritized. Valid older results remain available while replacements are prepared; failed refreshes do not erase them. Neither analysis estimates nor predicted alignment establish listening quality.

Pause freezes deck playback, effects, and preparation deadlines. Seek, manual navigation, and queue changes can cancel or replan transitions. Failed incoming files are skipped for the current session without deleting their queue entries; explicitly selecting a repaired entry retries it. Accepted schedules are protected from routine preparation and energy rearrangement. An estimate is not an accepted schedule. Audible automatic mixing, clipping/pitch/source-time listening checks, and macOS remote-command acceptance remain open in the current release record; automated or muted tests do not replace these checks.
