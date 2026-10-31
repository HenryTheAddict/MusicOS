# Lyrics failures and repeated refreshing — 2026-10-31

## Confirmed investigation

The local recording for 911 / Mr. Lonely is track 323, Tyler, The Creator, Flower Boy, 255 seconds. Its title/artist/album and duration are suitable for the documented signature lookup. No provider cache record existed for it. A separate known successful Static record (LRCLIB ID 2158256, 157 seconds versus the local 156 seconds) was already cached.

Live checks on this Mac returned HTTP 503 with the provider’s `ServerOverloaded` JSON and Retry-After: 1 for the exact 911 signature, structured title/artist search, and the known Static record ID. HTTP/1.1 and HTTP/2 both failed the same way. This establishes a provider-side service response during testing, not missing local audio or proof that the popular song has no lyrics. It does not establish a global outage or verify which 911 edition will match once the service recovers. Requests used the configured musicOS product identification; no lyrics or other data were uploaded.

LRCLIB documents 503 as temporary overload, 404 as no matching record, and 429 as rate limiting: https://lrclib.net/docs#errors. Its signature duration tolerance is ±2 seconds. No changes were made to recording matching or edition safeguards.

## App-side cause and repair

Previously, every active/inactive view change started local loading again and could restart online lookup. The same effect reset the allowance for two automatic 503 retries. Reopening or changing presentation could therefore repeat the attempt sequence and flash Finding even while the provider remained unavailable.

A bounded, shared 64-record session cache now coalesces load/lookup chains across expanded player, queue and mini-player. Keys include file/recording identity rather than artwork/theme state. Ready results are reused for five minutes, no-match results for 24 hours, and transient failure states for 30 seconds. A brief shared provider cooldown still permits local/cached lyrics to load first. Existing backend disk caches and explicit user lyric revisions retain precedence.

Countdowns only enable explicit Retry: no timer starts another network request. Explicit Retry respects Retry-After, and a filled error-state Retry action is available without opening a menu. Reopening does not reset the original cooldown. Existing content stays visible while a fresh result is checked. Inactive/unmounted views cannot replace the current song with late results. Lyric edits/Undo invalidate affected session records even if the player is closed; changes to the recording identity do not coalesce into stale work. No saved settings, playback commands, database schema, models, Android sync formats or audio tags were changed.

## Verification

189 UI unit tests passed, including provider cooldown, manual retry, repeated activation, cross-surface request coalescing, cached/local lyrics during overload, closed-player edit invalidation, changed recording identity and late-response protection. 19 browser interaction cases passed, including a busy-service fixture that performs one lookup across view changes, stays idle after its countdown, performs a second lookup only on explicit Retry, and reuses the successful result in the queue without playback commands. Existing lyric editing/Undo, seeking, reduced motion, mini-player bounds and menu tests remained green.

Provider availability cannot be repaired by a client release. The live exact 911 recording remains unverified while LRCLIB returns overload. Native release signature, unchanged protected runtime/model hashes, fresh paused/stopped installation check and preservation checks are recorded in the delivery record.
