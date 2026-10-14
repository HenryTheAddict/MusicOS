# MusicOS Labs beta

Connections live in Settings below the three appearance tiles. Both are disabled by default and remain optional. The Rust worker observes the authoritative local playback service; the frontend never sends file paths or handles provider sessions.

## Discord presence

Create a Discord developer application named MusicOS and enter its numeric Application ID in Settings. Discord desktop must be running. MusicOS uses local Discord IPC, publishes track title / artist / album and listening timestamps, clears presence on pause or disable, and retries a closed connection. There are no tokens to enter. macOS and Unix IPC are implemented; Windows named-pipe presence remains unverified and unavailable in this beta.

## Last.fm

This beta requires your own Last.fm application API key and shared secret. Enter them in Settings, authorize MusicOS in your browser, return to finish authorization, then explicitly enable scrobbling. Credentials and the resulting session are stored through the OS keychain, never returned to JavaScript or written to preferences. Reauthorization tokens remain in process memory only. Disconnect removes the keychain entry and clears queued listens.

Tracks longer than 30 seconds qualify after actual listening time reaches half the duration or four minutes, whichever is first. Pauses and seeks do not add listening time. Now Playing is best effort; completed listens are atomically queued before submission and retried every 30 seconds while enabled. The queue is bounded to 500 listens and associated with the authorized username. Disabling preserves the queue and stops further submissions. Last.fm declined listens are reported honestly.

Provider credentials were not supplied during implementation. Live authorization, account scrobbling, and a live Discord developer application have not been verified. Automated checks cover signature generation, listening accounting, IPC handshake/ping/acknowledgment/frame limits, and persistence. Windows/Linux native builds were not available on this macOS host.

## Visual implementation

Figtree variable font replaces Rodin in the React desktop build; its SIL Open Font License ships in the macOS bundle. Existing legacy font assets remain for the explicitly retained legacy implementations. The Sketch document at `/Users/henry/Pictures/Sketchy/musicosdesk.sketch` was changed with user authorization: all 258 text layers use Figtree with original font sizes, weights, and layer frames. Backup: `out/rebuild-qa/musicosdesk-before-figtree.sketch`.

The main panels use 16 interface-pixel radii with 60% smoothing; album artwork uses 100%, matching the reference layers. Responsive SVG clip paths use the same Bézier control points as the exported Sketch cover at 128×128. One ResizeObserver updates only resized surfaces; a bounded path cache avoids repeated preparation. The sidebar retains Sketch blue surfaces and custom icons while adding a MusicOS wordmark and clearer navigation spacing.

Playback progress uses Sketch's 4-pixel pale rail (#D6EFFD) and blue elapsed fill (#00A6FF), a larger invisible slider hit target, keyboard seeking, accessible elapsed/total time, and a drag draft immune to arriving playback snapshots. Pointer release commits one seek; failures remain visible. Reduced motion disables transitions. Column changes use finite 180 ms reflow transforms without delaying native window resizing.
