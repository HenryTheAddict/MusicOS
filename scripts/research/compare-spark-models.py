"""Isolated local comparison; never installs a model into MusicOS or writes its DB.
Run one model per process with the same --fixtures (an existing 24-song report).
Only public model/tokenizer artifacts are downloaded. Local audio stays local.
"""
import argparse, hashlib, json, os, resource, sqlite3, subprocess, time
from pathlib import Path
os.environ.update(HF_HUB_DISABLE_TELEMETRY='1',TOKENIZERS_PARALLELISM='false')
import numpy as np
import torch
from scipy.signal import resample_poly
from huggingface_hub import HfApi, snapshot_download
p=argparse.ArgumentParser();p.add_argument('--model',choices=['clap','muq','mert'],required=True);p.add_argument('--fixtures',type=Path,required=True);p.add_argument('--db',type=Path,required=True);p.add_argument('--clap',type=Path,required=True);p.add_argument('--official',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
a.output.mkdir(parents=True,exist_ok=True);os.chmod(a.output,0o700);torch.set_num_threads(2)
report={'model':a.model,'listening_acceptance':False,'packaged_acceptance':False,'threads':2,'excerpts':'Three identical 10-second mono excerpts at 0%, 50%, 85% of (duration - 10s); model-specific resampling only.'}
def save():
 path=a.output/f'{a.model}.json';path.write_text(json.dumps(report,indent=2)+'\n');os.chmod(path,0o600)
def unit(x):
 x=np.asarray(x,dtype=np.float32);return x/max(float(np.linalg.norm(x)),1e-12)
def snapshot(repo):
 if a.official:
  root=a.official/('models--'+repo.replace('/','--'));pinned=json.loads((root/'pinned.json').read_text());report.setdefault('inputs',[]).append(pinned);return root
 revision=HfApi().model_info(repo,token=False).sha
 report.setdefault('inputs',[]).append({'repo':repo,'revision':revision})
 return Path(snapshot_download(repo,revision=revision,token=False,cache_dir=str(a.output/'hub'),allow_patterns=['*.json','*.safetensors','*.bin','*.py','*.txt','*.model','LICENSE*','README.md']))
try:
 fixtures=json.loads(a.fixtures.read_text())['songs'];db=sqlite3.connect(f'file:{a.db}?mode=ro',uri=True);db.row_factory=sqlite3.Row
 songs=[]
 for old in fixtures:
  row=db.execute('select id,path,title,artist,album,duration,favorited from tracks where id=? and deleted_at is null',(old['id'],)).fetchone()
  if row is None or (row['title'],row['artist'],row['album'])!=(old['title'],old['artist'],old['album']) or not Path(row['path']).is_file():raise RuntimeError('Fixture identity changed; comparison stopped instead of silently substituting songs')
  if old.get('sha256') and hashlib.file_digest(open(row['path'],'rb'),'sha256').hexdigest()!=old['sha256']:raise RuntimeError('Fixture content hash changed')
  songs.append(dict(row))
 report['fixture_ids']=[s['id'] for s in songs];report['fixture_count']=len(songs)
 cache=a.output/'excerpts';cache.mkdir(exist_ok=True);os.chmod(cache,0o700)
 started=time.perf_counter();prompts=[f'Music that sounds {mood}.' for mood in ['chilled','energetic','melancholic','bright','focused','dreamy']];mood_vectors=None
 if a.model=='clap':
  from transformers import ClapModel,ClapFeatureExtractor,AutoTokenizer
  model=ClapModel.from_pretrained(a.clap,local_files_only=True).eval();processor=ClapFeatureExtractor.from_pretrained(a.clap,local_files_only=True);tokenizer=AutoTokenizer.from_pretrained(a.clap,local_files_only=True)
  with torch.inference_mode():mood_vectors=model.get_text_features(**tokenizer(prompts,padding=True,return_tensors='pt')).numpy()
  def embed(wave):
   with torch.inference_mode():return model.get_audio_features(**processor(wave,sampling_rate=48000,return_tensors='pt')).numpy()[0]
  rate=48000;report['inputs']=[{'repo':'laion/larger_clap_general','revision':'ada0c23a36c4e8582805bb38fec3905903f18b41'}]
 elif a.model=='muq':
  from muq import MuQMuLan,MuQ
  from transformers import XLMRobertaModel,XLMRobertaConfig
  from unittest.mock import patch
  root=snapshot('OpenMuQ/MuQ-MuLan-large');audio_root=snapshot('OpenMuQ/MuQ-large-msd-iter');text_root=snapshot('FacebookAI/xlm-roberta-base')
  config=json.loads((root/'config.json').read_text());audio_config=json.loads((audio_root/'config.json').read_text());config['text_model']['name']=str(text_root.resolve())
  # The joint checkpoint includes both base encoders. Construct their official
  # architectures from pinned configs, then require a strict full-state load;
  # downloading the same base weights again would waste another 2+ GB.
  with patch.object(MuQ,'from_pretrained',side_effect=lambda *args,**kwargs:MuQ(audio_config)),patch.object(XLMRobertaModel,'from_pretrained',side_effect=lambda *args,**kwargs:XLMRobertaModel(XLMRobertaConfig.from_pretrained(text_root,local_files_only=True))):
   model=MuQMuLan(config)
  weights=torch.load(root/'pytorch_model.bin',map_location='cpu',weights_only=True,mmap=True);model.load_state_dict(weights,strict=True);del weights;model.eval();report['strict_weight_loading']=True
  with torch.inference_mode():mood_vectors=model(texts=prompts).numpy()
  def embed(wave):
   with torch.inference_mode():return model(wavs=torch.from_numpy(wave.copy()).unsqueeze(0)).numpy()[0]
  rate=24000
 else:
  from transformers import AutoModel,AutoFeatureExtractor
  root=snapshot('m-a-p/MERT-v1-95M');model=AutoModel.from_pretrained(root,trust_remote_code=True,local_files_only=True).eval();processor=AutoFeatureExtractor.from_pretrained(root,local_files_only=True)
  def embed(wave):
   with torch.inference_mode():return model(**processor(wave,sampling_rate=24000,return_tensors='pt'),output_hidden_states=True).last_hidden_state.mean(dim=1).numpy()[0]
  rate=24000;report['mood_support']='No joint text encoder; mood matching is not comparable and is not fabricated.'
 report['load_seconds_including_first_download']=time.perf_counter()-started;vectors=[];timings=[]
 for index,song in enumerate(songs):
  base=cache/f'{song["id"]}.npz';signature=hashlib.sha256(f'{song["path"]}:{Path(song["path"]).stat().st_size}:{Path(song["path"]).stat().st_mtime_ns}'.encode()).hexdigest()
  if base.exists():
   data=np.load(base);assert str(data['signature'])==signature,'Fixture audio changed';waves=data['audio']
  else:
   duration=song['duration'];starts=[0.,max(0,duration-10)*.5,max(0,duration-10)*.85] if duration>20 else [0.]*3;waves=[]
   for at in starts:
    raw=subprocess.run(['/opt/homebrew/bin/ffmpeg','-v','error','-ss',str(at),'-i',song['path'],'-t','10','-ac','1','-ar','48000','-f','f32le','pipe:1'],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE).stdout
    wave=np.frombuffer(raw,dtype='<f4').copy();wave=np.pad(wave,(0,max(0,480000-len(wave))))[:480000];waves.append(wave)
   waves=np.asarray(waves);np.savez(base,audio=waves,signature=signature);os.chmod(base,0o600)
  started=time.perf_counter();values=[unit(embed(wave if rate==48000 else resample_poly(wave,1,2).astype(np.float32))) for wave in waves];vectors.append(unit(np.mean(values,axis=0)));timings.append(time.perf_counter()-started)
  print(f'{a.model}: {index+1}/{len(songs)} songs',flush=True)
 vectors=np.asarray(vectors);np.save(a.output/f'{a.model}-embeddings.npy',vectors);os.chmod(a.output/f'{a.model}-embeddings.npy',0o600)
 def rank(seed=None,mood=None):
  selected=[] if seed is None else [seed];remaining=[i for i in range(len(songs)) if i not in selected]
  while len(selected)<min(6,len(songs)):
   def score(i):
    base=float(vectors[i]@vectors[seed]) if seed is not None else float(vectors[i]@mood)
    artist=min(.12,sum(songs[j]['artist']==songs[i]['artist'] for j in selected)*.04);album=min(.075,sum((songs[j]['artist'],songs[j]['album'])==(songs[i]['artist'],songs[i]['album']) for j in selected)*.025)
    return base-artist-album+(.025 if songs[i]['favorited'] else 0)
   chosen=max(remaining,key=lambda i:(score(i),-songs[i]['id']));remaining.remove(chosen);selected.append(chosen)
  return selected
 seeds=list(range(min(6,len(songs))));report['seed_results']=[{'seed':songs[i]['id'],'track_ids':[songs[j]['id'] for j in rank(i)],'distinct_artists_top5':len({songs[j]['artist'] for j in rank(i)[1:6]})} for i in seeds]
 if mood_vectors is not None:
  mood_vectors=np.asarray([unit(v) for v in mood_vectors]);report['mood_cosines']=(mood_vectors@mood_vectors.T).tolist();report['mood_results']=[{'prompt':prompt,'track_ids':[songs[j]['id'] for j in rank(mood=vector)]} for prompt,vector in zip(prompts,mood_vectors)]
 report.update(dimensions=int(vectors.shape[1]),median_seconds_per_song=float(np.median(timings)),p95_seconds_per_song=float(np.percentile(timings,95)),peak_process_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,timings_seconds=timings,status='objective_comparison_complete')
except Exception as error:report.update(status='failed',error=f'{type(error).__name__}: {error}');raise
finally:save()
