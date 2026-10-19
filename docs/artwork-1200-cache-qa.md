# 1200 px photo cache — 2026-10-18

Implemented a persistent 1200 px display master, with 640 px grid and 128 px small/palette renditions. Legacy 1280 requests map to 1200. Opaque images use quality-90 JPEG; real alpha uses lossless PNG. On macOS, ImageIO prepares an orientation-correct bounded image before JPEG/PNG encoding; there is no second full-size Rust decode. No new codecs or runtime dependencies. JPEG is supported by the current desktop image pipeline; JPEG XL was not introduced.

Keys include canonical original path, file size, modification time, and encoding version. Writes publish atomically after flushing. A shared worker avoids duplicate conversions; corrupt cache headers are regenerated. Originals, canonical artwork identities, full-resolution viewer URLs, saved palettes, collections, listening settings, and the separate depth RAM/texture budgets remain intact.

The native Cover no longer assigns the enormous original to its initial image URL. It requests bounded artwork only near the viewport, deduplicates concurrent requests, ignores obsolete resize completions, and retains a valid decoded frame while its replacement prepares. Portrait framing also returns bounded cached display images; its original photo remains available separately. Hidden retained views stop initiating display-cache work.

## Measurements

Production rendition/palette functions, disposable copy of the actual Fancy That cover (4000×4000 PNG, 29,076,484 bytes):

- 1200×1200 JPEG master: 475,428 bytes (98.4% smaller encoded file).
- 640×640 JPEG: 158,319 bytes; 128×128 JPEG: 9,836 bytes.
- Debug first master plus grid conversion: 1,573.3 ms. This is a cold cost, not a speedup claim.
- Debug repeat master/grid/palette-thumbnail disk lookups: 0.04–0.10 ms; palette sampling from the prepared cache: 8.51 ms.
- Original hash unchanged. A 1200 px RGBA decode is approximately 5.5 MiB versus 61 MiB for this 4000 px original.

Release measurements using the same production functions and disposable original: first master plus grid 288.36 ms; first 128 px derivation 21.76 ms; repeat disk lookups 0.026–0.054 ms; palette sampling 5.99 ms. These are single-run cache-operation measurements, not an overall launch benchmark or frame-rate claim.

Raw measurements: `out/persistent-qa/artwork-cache-measurements.json` (debug), `out/persistent-qa/artwork-cache-release.json` (release).

## Verification

- Seven Rust rendition/upgrade tests pass: EXIF orientation, framing, transparency, no upscaling, compressed shared master, concurrent publication, corrupt header recovery, changed source revisions, unchanged originals/canonical references.
- Nine provider tests pass, including large artwork sampling and readable/deterministic palettes.
- All 98 UI unit tests pass, including bounded initial URLs, off-screen deferral, deduplicated requests, resize races, retained frames and existing artwork fallbacks.
- UI TypeScript and release frontend build pass.

Thirteen browser checks pass across the final depth/navigation suite plus dark-corner checks at 1×/2×. Verified native Albums, Artists, Fancy That adaptive colors, and the expanded artwork viewer in the signed package. Returned to Albums. The update was loaded paused at The Recipe 324.3 sec; the user subsequently selected Go! by M83, which remained paused at 62.5 sec throughout final checks. 507 songs and 17 favorites remain visible. No changes to playback/queue or library records were made by this cache task.

Ad-hoc signed executable SHA256: `49713afb24b58c2e72e59915c0a3eb5c5a980d516a3979a3adb2549703ec747b`. Strict deep signature verification passed; all 12 current UI chunks were verified embedded. This is not a Developer ID/notarized package.

Unchanged embedded runtime SHA256:
- Playback: `6b1fa66db8e7c1fa1723a210018d9d2f729e76cc907357e3201b2a2f4ba80e1d`.
- ONNX: `ff52828ce7c6dd1aea99a5e9f5da9e2786e3d03e467fbf8b6893a4b624334920`.

 Human assessment of remaining motion/performance complaints, touchscreen behavior, and Windows/Linux are separate gates.
