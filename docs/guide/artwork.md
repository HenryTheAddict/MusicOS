# Artwork, portraits, and depth

[Guide home](README.md)

Albums use one canonical image across cards, songs, Audio, Now Playing, and adaptive colors. Source tags and embedded images are preserved. Missing artwork uses a consistent placeholder; failures should retain a previous valid cover.

Album covers stay still when Depth artwork is off. With depth enabled, grid covers, album headers and expanded images respond gently to pointer position without rotating the surrounding box. Click the large cover inside an album, or an artist portrait on its page, to expand the original image. Close with Escape, the close button, or the surrounding backdrop. The viewer uses a stationary blurred, darkened image backdrop with a readable caption. Artist statistics appear on hover or keyboard focus and dismiss independently.

## Portrait framing

On macOS, Apple Vision analyzes an oriented local image with face landmarks. Framing keeps headroom and similarly prominent faces together. Missing or uncertain detection retains original framing. No extra face-model download is needed. Group photos, masks, illustrations, and unusual angles can still defeat detection; the full-size viewer shows the original photo.

## Optional depth

Open **Settings → Appearance → Artwork motion**. Artwork parallax and Depth artwork are optional. Depth requires installing the pinned Apple Core ML Depth Anything V2 Small F32 package (99,188,950 bytes). F32 changes precision, not parameter count. Installation has progress, cancellation, retry, removal, and attribution.

Ordinary images stay visible while one background worker prepares maps. Maps are cached on disk against final image content and processing/model version. A new crop or changed image produces a new map; moving an identical image does not. Invalid cache images are rebuilt. A newly computed map gets one short completion reveal; cached maps, scrolling and subsequent hovers do not replay it. A bounded memory cache avoids repeatedly decoding visible covers. Removal clears the depth installation and its maps; it does not remove music or original artwork.

Reduced motion disables depth movement and elastic motion. Graphics/context failures fall back to normal artwork. Performance depends on display, window size, scale, and GPU. Measured callback cadence is not a guarantee that every frame was presented at 120 fps.

## Gaussian splats

An “Insane depth” product toggle is not included. Apple's SHARP model license excludes product development, and its reference video renderer requires CUDA. Research feasibility and a product-eligible model/native renderer must be resolved before exposing a functional toggle. See [Apple SHARP](https://github.com/apple/ml-sharp) and its [model license](https://github.com/apple/ml-sharp/blob/main/LICENSE_MODEL).

Expanded artwork waits for the opening animation to finish before measuring its depth overlay, so the source image and overlay share their final bounds. The viewer carries the source artwork version; explicit artwork invalidation clears prepared memory textures while valid disk maps remain reusable. Opening, scrolling, or resizing an unchanged image does not regenerate a valid cached map.
