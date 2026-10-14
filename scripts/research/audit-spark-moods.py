"""Development-only diagnostics; this never approves or publishes a checkpoint."""
import json
import math
from pathlib import Path

root = Path(__file__).resolve().parents[2] / 'out/rebuild-qa/model-evaluation'
vectors = json.loads((root / 'mood-vectors.json').read_text())
evaluation = json.loads((root / 'local-evaluation-report.json').read_text())
def cosine(a, b):
    return sum(x*y for x, y in zip(a, b)) / math.sqrt(sum(x*x for x in a) * sum(y*y for y in b))

pairs = [{'a': a, 'b': b, 'cosine': cosine(vectors[a], vectors[b])}
         for i, a in enumerate(vectors) for b in list(vectors)[i+1:]]
orders = {tuple(mix['track_ids']) for mix in evaluation['moods']}
report = {
    'checkpoint': evaluation['checkpoint']['checkpoint'],
    'revision': evaluation['checkpoint']['revision'],
    'pairwise_mood_cosines': pairs,
    'distinct_mood_sequences': len(orders),
    'moods_tested': len(evaluation['moods']),
    'blocking_observation': 'Several moods produced identical sequences; inspect text embeddings and listening relevance.' if len(orders) < len(evaluation['moods']) else None,
    'interpretation': 'Near-identical text vectors and identical rankings need investigation. Runtime parity alone does not establish music relevance.',
    'production_approved': False,
    'blind_listening_complete': False,
}
(root / 'mood-separation-report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'distinct_mood_sequences': len(orders), 'minimum_pair_cosine': min(p['cosine'] for p in pairs), 'production_approved': False}))
