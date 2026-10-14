# Sparklist model evaluation — work in progress

No production model has been selected. Audio mix generation stays disabled until
music listening relevance and packaged runtime verification are complete.

## Candidates and licenses

- Music-focused [LAION larger_clap_music](https://huggingface.co/laion/larger_clap_music),
  revision `a0b4534a14f58e20944452dff00a22a06ce629d1`: checkpoint model card
  specifies Apache-2.0. This is separate from the [CLAP code license](https://github.com/LAION-AI/CLAP).
- Relevant Essentia candidates: Discogs EffNet embedding plus its mood classification
  heads. The [licensing page](https://essentia.upf.edu/licensing_information.html)
  documents noncommercial/no-derivatives weight terms and commercial licensing.
  The [model index](https://essentia.upf.edu/models.html) contains differing general
  language; exact checkpoint permission must be established separately. These
  candidates are not approved for redistribution in MusicOS.
- Metadata-only baseline is not yet measured and cannot stand in for audio similarity.

## Measured runtime prototype

An isolated developer-only Python environment exported the official music checkpoint
into an ONNX audio encoder/projector. This adds no Python requirement to the app.
The Rust experiment uses `ort 2.0.0-rc.13`, dynamically loading ONNX Runtime 1.31.0
on Apple Silicon macOS 27.2. Input is the checkpoint processor's 48 kHz audio/mel
representation, shape `[1,1,1001,64]`, output is a normalized 512-dimensional vector.
Synthetic input tests correctness/runtime only; no private audio or embeddings uploaded.

- Original weights: 776,444,665 bytes; converted ONNX: 277,114,827 bytes.
- Python ONNX runs per ten-second input: 49.0, 40.7, 40.1 ms.
- Rust ONNX runs: 177.9, 104.4, 83.8 ms.
- Python export/reference process peak RSS: 1,784,004,608 bytes. This includes Torch
  and ONNX together and is not a packaged inference memory measurement.
- Python ONNX vs Torch max error: `1.527e-7`, cosine `1.0`.
- Rust vs reference max error: `0.000174889`, cosine `0.999999417`.
  The initial stricter `1e-4` absolute check failed. The prototype now explicitly
  checks cosine > .99999, absolute error < .001 and 512 dimensions. Different
  optimized execution paths have small component differences; cosine parity does
  not establish playlist relevance.

Scripts: `scripts/research/evaluate-clap.py`, `scripts/research/clap-runtime/`.
Raw results: `out/rebuild-qa/model-evaluation/runtime-report.json`, `rust-runtime.log`.

Still required: blind seed/mood listening comparison, metadata baseline, artist
clustering assessment, exact packaged-memory/speed measurements, Rust preprocessing,
licensed distributable model artifact and delivery URL, download/cancel/retry/removal,
versioned embedding worker and invalidation, feedback/generation storage, offline
packaged macOS verification during uninterrupted real local playback. Current BPM/key
DSP supports DJmode and is explicitly not Sparklist embedding analysis.

## October 10 follow-up: mood discrimination blocks selection

The existing local evaluation already exercises Rust preprocessing, representative
file excerpts, embedding persistence and deterministic ranking on 24 local songs.
These checks supersede the earlier statement that Rust preprocessing was missing;
they do not establish packaged playback concurrency or listening relevance.

Re-running the development audit (`scripts/research/audit-spark-moods.py`) found
that five moods produce the same ordered sequence; Focused differs. Every
pair of the current mood vectors has cosine similarity above 0.999. A separate
local text-encoder check also produced similarities above 0.999 between unrelated
music, dog and engine prompts, despite no missing or mismatched checkpoint keys.
This is a concrete representation/evaluation concern requiring investigation,
not evidence of successful mood selection. No normalization workaround or metadata
filter is being substituted to claim audio generation works.

Diagnostics: `out/rebuild-qa/model-evaluation/mood-separation-report.json`.
The staged artifact remains an evaluation candidate, with no download URL. Model
selection, blind listening, packaged runtime/playback checks and reviewed hosting
are still open. Production Sparklist generation stays unavailable.

## Replacement candidate investigation

The collapsed text embeddings were reproduced directly from checkpoint tensors
with no missing, unexpected or mismatched keys. The [upstream benchmark issue](https://github.com/embeddings-benchmark/mteb/issues/5069)
and [model discussion](https://huggingface.co/laion/larger_clap_music/discussions/2)
report the same problem. This rules out simply enabling the existing artifact as
an acceptable mood generator.

An alternative [LAION larger_clap_general checkpoint](https://huggingface.co/laion/larger_clap_general/tree/ada0c23a36c4e8582805bb38fec3905903f18b41)
(revision `ada0c23a36c4e8582805bb38fec3905903f18b41`) has an Apache-2.0
checkpoint model card and is trained on general audio, music and speech. It is
being evaluated as a replacement, not declared the winner.

- The six mood prompt vectors have pairwise cosine range 0.492885–0.930698,
  compared with >0.999 for the broken music candidate.
- Exported audio ONNX: 281,165,749 bytes; staged model plus configuration/moods:
  281,234,043 bytes.
- Synthetic ONNX/Torch maximum absolute error: 0.000001382; cosine ~1.0.
- Python ONNX ten-second excerpt inference: 42.0 / 37.5 / 36.9 ms.
- Peak export/reference process RSS: 1,906,212,864 bytes, including Torch and
  ONNX; this is not packaged app memory.
- The staged manifest remains `evaluation_candidate` with no public download URL.
  Rust local-file ranking and later listening/native checks remain separate gates.

Reproduction scripts now accept explicit checkpoint, revision, and output paths
so candidates do not overwrite each other's artifacts. Staging verifies the
checkpoint's Apache-2.0 model-card declaration and never changes approval status.

The replacement's Rust local-file evaluation completed on the same 24-song set:
all six moods produced different ordered sequences; the median decode/preprocess/
inference time for three representative excerpts was 0.337826 seconds per song.
Each of the six seed comparisons had five distinct artists in its first five
recommendations. Raw results are in
`out/rebuild-qa/model-evaluation-general/local-report.json`; compact results are
in `summary.json`. This is objective generation/diversity evidence, not blind
listening relevance, packaged acceptance, or approval for public release.
The user approved an explicitly opt-in experimental beta, default off. Only the pinned general checkpoint and exact artifact checksums are accepted; listening and packaged-runtime acceptance remain required before promotion.
