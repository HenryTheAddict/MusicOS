# Separate Sparklist comparison — October 24, 2026

This is a local research comparison, not a model replacement or an approval for
production distribution. The desktop release retains its installed CLAP model,
existing embeddings, consent, saved mixes and model/runtime hashes.

## Method

The original evaluation's 24 song titles/artists/albums were resolved against the
current library because archived database IDs no longer identify those songs.
The new fixture manifest pins current IDs and SHA-256 file hashes; one duplicated
metadata match uses the lowest current ID and is recorded. All three candidates
use exactly this same frozen set. The production database is opened read-only.

Each song supplies three ten-second excerpts at the start, middle and 85% point of
the available start range. Decode is cached once at 48 kHz mono; music models get
24 kHz resampling. Unit-normalized excerpt vectors are averaged and normalized.
Seed ranking uses the existing cosine/favorite/diversity policy. MuQ-MuLan and
CLAP receive the same six mood prompts. MERT has no joint text encoder here, so
no mood capability is fabricated.

Official snapshots and LFS checksums are pinned. The MuQ joint checkpoint contains
its base encoders: their official architectures are constructed from pinned configs
and all joint weights load strictly, avoiding redundant base-weight downloads.
An isolated virtual environment uses two Torch CPU threads; no Python dependency
is added to MusicOS. No local audio or embeddings are uploaded.

## Measured prototypes

| Candidate | Dimensions | Median / song | p95 / song | Peak process RSS |
|---|---:|---:|---:|---:|
| Current general CLAP | 512 | 0.198 s | 0.251 s | 1.08 GiB |
| MuQ-MuLan large | 512 | 1.627 s | 1.953 s | 3.78 GiB |
| MERT v1 95M, final-layer mean | 768 | 0.932 s | 1.021 s | 1.32 GiB |

These are isolated CPU prototype times for three excerpts, including model-specific
resampling/inference and excluding the shared decode cache. Peak RSS includes the
whole evaluation process, not just model weights. They are not packaged app speed
or memory measurements. Model size and representation differ; MERT layer selection
has not been tuned for this library.

All six seed comparisons retain five distinct artists in their next five songs
under the same ranking policy. CLAP and MuQ each produce six distinct mood orders.
Diversity and different orders establish neither listening relevance nor a winner.
MuQ costs about eight times CLAP's preparation time in this run; keep the current
model until blind relevance and packaged-runtime results justify a change.

## Listening and licensing gates

A private offline worksheet randomizes model labels for each seed and uses the
same middle ten-second audio clips for all candidates. It provides local ratings
and export, with no autoplay or network calls. It is at
`out/rebuild-qa/spark-comparison/results/listening-review/review.html` in the isolated
checkout; the separate `key.json` reveals model identities after rating.

**Blind listening is prepared, not completed.** No human ratings, quality winner,
ONNX/Core ML export acceptance or production promotion is claimed.

MuQ-MuLan and MERT released weights use CC BY-NC 4.0. Research availability does not
make them eligible for unrestricted bundled distribution. Sources:
[MuQ official project](https://github.com/tencent-ailab/muq),
[MuQ-MuLan model](https://huggingface.co/OpenMuQ/MuQ-MuLan-large),
[MERT v1 95M](https://huggingface.co/m-a-p/MERT-v1-95M).

Scripts: `compare-spark-models.py`, `fetch-comparison-model.py`, and
`spark-listening-review.py` under `scripts/research/`. Raw revisions, hashes,
rankings and measurements remain in ignored private evaluation output.
