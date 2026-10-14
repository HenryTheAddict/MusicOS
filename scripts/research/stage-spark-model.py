"""Stage the evaluated candidate for review. Never publishes or approves it."""
import argparse, hashlib, json, re, shutil
from pathlib import Path
root = Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser()
parser.add_argument('--source',default='out/rebuild-qa/model-evaluation')
parser.add_argument('--id',default='clap-music-v1')
parser.add_argument('--checkpoint',default='laion/larger_clap_music')
parser.add_argument('--revision',default='a0b4534a14f58e20944452dff00a22a06ce629d1')
options=parser.parse_args()
assert options.id and all(c.isalnum() or c=='-' for c in options.id), 'Use a simple model identifier'
source = root / options.source
card=(source/'music-checkpoint/README.md').read_text()
frontmatter=card.split('---',2)
if len(frontmatter)<3 or not re.search(r'^license:\s*apache-2\.0\s*$',frontmatter[1],re.MULTILINE):
    raise RuntimeError('This staging script requires an Apache-2.0 checkpoint model card; verify other licenses separately.')
stage = source / 'release-candidate' / options.id
stage.mkdir(parents=True, exist_ok=True)
files = []
for origin, name in [(source/'music-audio.onnx','audio.onnx'),(source/'mood-vectors.json','moods.json'),(source/'music-checkpoint/preprocessor_config.json','preprocessor_config.json')]:
    target=stage/name
    shutil.copyfile(origin,target)
    files.append({'name':name,'bytes':target.stat().st_size,'sha256':hashlib.sha256(target.read_bytes()).hexdigest()})
manifest = {'schema':1,'id':options.id,'checkpoint':options.checkpoint,'revision':options.revision,'license':'Apache-2.0','license_url':f'https://huggingface.co/{options.checkpoint}/tree/{options.revision}','preprocessing':'clap-music-slaney-48k-1024-480-repeatpad-v1','runtime':'onnxruntime-1.31.0','dimensions':512,'status':'evaluation_candidate','download_base':None,'files':files}
(stage/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(stage/'REVIEW.txt').write_text('Evaluation candidate; not approved for production.\nBlind listening and packaged playback/concurrency evaluation remain required.\nPublication requires review of this staged artifact. No audio or embeddings are included.\n')
print(json.dumps({'stage':str(stage),'total_bytes':sum(f['bytes'] for f in files)},indent=2))
