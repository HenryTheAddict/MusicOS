# Standard-brightness photo decoding

25 October 2026. Desktop-only follow-up to the clarification that certain photos,
rather than the entire window, look HDR-like. No further theme changes are made.

## What changed

The previous document/media range limit remains useful, but is not a substitute
for correct photo decoding. Expanded artwork still selected the original file
instead of the prepared display rendition, and expanded depth loaded its original
photo separately. Artist framing also discarded the original color profile before
converting its cropped output. These paths could present photos with different
color handling across their ordinary, expanded and moving states.

A host-only macOS ImageIO bridge explicitly requests SDR decoding on macOS 14+,
draws into an 8-bit sRGB bitmap, and targets headroom 1 on macOS 15+. Older macOS
uses the sRGB bitmap conversion. The bridge is linked into the desktop host;
the embedded playback runtime and models are not rebuilt or modified. Apple
provides the [SDR decode request](https://developer.apple.com/documentation/imageio/kcgimagesourcedecodetosdr)
and documents [preferred dynamic range / tone mapping](https://developer.apple.com/documentation/coregraphics/adopting-advancements-in-hdr-image-rendering).

Album display copies and artist framing use this conversion before Rust reads or
crops them. Color conversion preserves the source profile's meaning rather than
retagging its samples as sRGB. The display and framing cache identities are revised
so existing derived copies cannot bypass the corrected decoder. Original artwork,
metadata, depth maps, model files and audio embeddings remain intact. Only requested
visible display copies are prepared; this does not batch-process the library.

Expanded artwork starts with the already visible display image and requests a
cached, bounded 2048 px copy. It decodes before swapping; failures keep the visible
copy. Late completions cannot replace a different photo or reopen a dismissed
viewer. A larger copy does not reopen the dialog or replay its entrance. Modal
lifetime follows open/closed state, with focus returning to the latest opener.
Moving artwork samples that same displayed image, including expanded views, instead
of loading a second original photo. Source identity and existing depth maps remain.

The ordinary 128/640/1200 display sizes and legacy 1280 alias retain their bounded
1200 master. The 2048 master is created only for an expanded-view request. Original
bytes, orientation, framing, alpha, aspect ratio and source-size safeguards are
preserved. Preview/thumbnail requests are deduplicated and obsolete UI completions
are ignored; a closed viewer does not begin an unnecessary image decode.

## Verification

Synthetic Display P3 and 16-bit ITU-R 2100 PQ PNG fixtures are generated locally
with public Core Graphics/ImageIO APIs; their source and reproduction commands are
in `src-tauri/tests/fixtures/`. No user photographs are used as test fixtures.
Color checks compare the P3-to-sRGB result against converted channel values, verify
SDR mapping of PQ swatches into 8-bit output, preserve normal sRGB colors, and check
original bytes remain unchanged. Native tests also cover cache reuse, transparency,
EXIF rotation, bounded large copies and concurrent cache publication.

UI checks cover visible-copy startup, decode-before-swap, retained fallback,
request deduplication, stale replies, closing during preparation, a different photo
opened during preparation, no dialog re-entry, and focus restoration. Browser
checks reject a deliberately substituted raw original for both expansion and
depth, and inspect the viewer at 720×480 / 1280×800 across all four interface scales
with Reduced Motion. Existing hover bounds, context recovery, no preparation flash,
idle drawing and expanded depth alignment are checked.

Build identity and native delivery results are recorded below after installation.
Screenshots cannot establish physical HDR luminance. The unknown specific photos
mentioned by the user and human display acceptance remain a separate visual gate;
the decoder contract is tested with actual wide-gamut / HDR-tagged input files.
