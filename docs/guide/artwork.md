# Artwork, portraits, and depth

[Guide home](README.md)

Albums use one canonical image across cards, songs, Audio, Now Playing, and adaptive colors. Source tags and embedded images are preserved. Missing artwork uses a consistent placeholder; failures should retain a previous valid cover.

Album covers stay still when Depth artwork is off. With depth enabled, grid covers, album headers and expanded images respond to pointer position by at most 3 px, with an additional scroll shift capped at 1.5 px and a quick return to neutral. The surrounding box does not rotate or zoom. Click the large cover inside an album, or an artist portrait on its page, to expand the original image. Close with Escape, the close button, or the surrounding backdrop. The viewer uses a stationary blurred, darkened image backdrop with a readable caption. Artist statistics appear on hover or keyboard focus and dismiss independently.

## Portrait framing

On macOS, Apple Vision analyzes an oriented local image with face landmarks. Framing keeps headroom and similarly prominent faces together. Missing or uncertain detection retains original framing. No extra face-model download is needed. Group photos, masks, illustrations, and unusual angles can still defeat detection; the full-size viewer shows the original photo.

## Optional depth

Open **Settings → Appearance → Artwork motion**. Depth artwork is optional. Ordinary scrolling uses native momentum and edge bounce; covers do not tilt or stretch with scrolling. Depth requires installing the pinned Apple Core ML Depth Anything V2 Small F32 package (99,188,950 bytes). F32 changes precision, not parameter count. Installation has progress, cancellation, retry, removal, and attribution.

Ordinary images stay visible while one background worker prepares maps. Maps are cached on disk against final image content and processing/model version. A new crop or changed image produces a new map; moving an identical image does not. Invalid cache images are rebuilt. A newly computed map gets one short completion reveal; cached maps, scrolling and subsequent hovers do not replay it. A bounded memory cache avoids repeatedly decoding visible covers. Removal clears the depth installation and its maps; it does not remove music or original artwork.

Reduced motion disables depth movement and completion animation. Graphics/context failures fall back to normal artwork. Performance depends on display, window size, scale, and GPU. Measured callback cadence is not a guarantee that every frame was presented at 120 fps.

## Gaussian splats

An “Insane depth” product toggle is not included. Apple's SHARP model license excludes product development, and its reference video renderer requires CUDA. Research feasibility and a product-eligible model/native renderer must be resolved before exposing a functional toggle. See [Apple SHARP](https://github.com/apple/ml-sharp) and its [model license](https://github.com/apple/ml-sharp/blob/main/LICENSE_MODEL).

Expanded artwork waits for the opening animation to finish before measuring its depth overlay, so the source image and overlay share their final bounds. The viewer carries the source artwork version; explicit artwork invalidation clears prepared memory textures while valid disk maps remain reusable. Opening, scrolling, or resizing an unchanged image does not regenerate a valid cached map.

## Depth residency

Depth maps remain on disk. Shared prepared images use a 256 MiB RAM budget, while active compositors share 128 MiB of texture memory. Visible images are pinned; nearby artwork is prefetched and off-screen resources are evicted in least-recently-used order when necessary. An additional bounded 64 MiB response cache avoids repeated map transfers.

Ordinary resizing preserves the compositor and updates its geometry. After GPU context loss, normal artwork remains visible while prepared resources restore depth. Reusing cached maps does not repeat the completion reveal or run inference again. Scrolling uses native edge behavior rather than synthetic page stretching or blur.

## Full-resolution images

Embedded and manual originals are retained. Ordinary artwork uses a persistent display cache capped at 1200 pixels on its longest edge: quality-90 JPEG for opaque photographs, lossless PNG for transparency. Grid and palette thumbnails reuse that master instead of decoding the original again. Near-visible views request the appropriate size, retain already decoded artwork during updates, and share in-flight requests. The expanded viewer still uses the original and sharper depth textures. Images are never upscaled. A larger catalog rendition is used automatically only for the same verified image asset. Different artwork remains a reviewable suggestion.

## Appearance

**Settings → Appearance** provides System, Light and Dark modes, plus Artwork, Original blue and Custom color sources. Custom controls change accent, saturation and surface tint with a live preview and Reset. Dark mode uses charcoal surfaces and colored accents. Theme controls do not change saved artwork palettes. Existing installations retain their previous light appearance; new installations start with System. Surfaces stay solid, and the launch color fade respects Reduced Motion.
