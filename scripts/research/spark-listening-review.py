"""Build a private, offline blind listening worksheet from comparison results.
No app database writes, network calls, autoplay or model changes.
"""
import argparse,html,json,os,random,wave
from pathlib import Path
import numpy as np
p=argparse.ArgumentParser();p.add_argument('results',type=Path);p.add_argument('fixtures',type=Path);a=p.parse_args();root=a.results/'listening-review';root.mkdir(exist_ok=True);os.chmod(root,0o700);clips=root/'clips';clips.mkdir(exist_ok=True)
songs={s['id']:s for s in json.loads(a.fixtures.read_text())['songs']};models={name:json.loads((a.results/f'{name}.json').read_text()) for name in ['clap','muq','mert']};assert all(r['status']=='objective_comparison_complete' for r in models.values())
for id in songs:
 audio=np.load(a.results/'excerpts'/f'{id}.npz')['audio'][1];target=clips/f'{id}.wav'
 with wave.open(str(target),'wb') as output:output.setnchannels(1);output.setsampwidth(2);output.setframerate(48000);output.writeframes((np.clip(audio,-1,1)*32767).astype('<i2').tobytes())
 os.chmod(target,0o600)
rng=random.Random(20261024);cards=[];key={}
for index,seed in enumerate(models['clap']['seed_results']):
 order=list(models);rng.shuffle(order);seed_id=seed['seed'];group=[];key[str(seed_id)]={}
 for position,name in enumerate(order):
  label=chr(65+position);key[str(seed_id)][label]=name;ids=next(r['track_ids'] for r in models[name]['seed_results'] if r['seed']==seed_id)
  items=''.join(f'<li><span>{html.escape(songs[id]["title"])} — {html.escape(songs[id]["artist"])}</span><audio controls preload="none" src="clips/{id}.wav"></audio></li>' for id in ids)
  group.append(f'<article><h3>Mix {label}</h3><ol>{items}</ol><label>Fit to the seed <select data-rating="{seed_id}-{label}"><option value="">Not rated</option>'+''.join(f'<option>{n}</option>' for n in range(1,6))+'</select></label></article>')
 cards.append(f'<section><h2>Starting from {html.escape(songs[seed_id]["title"])}</h2><div class="mixes">'+''.join(group)+'</div></section>')
(root/'key.json').write_text(json.dumps(key,indent=2));os.chmod(root/'key.json',0o600)
page='''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>MusicOS blind mix comparison</title><style>body{font:16px system-ui;margin:24px;background:#202329;color:#edf2f8;line-height:1.5}main{max-width:1400px;margin:auto}h1,h2,h3{line-height:1.2}.mixes{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:16px}article{background:#30343b;padding:16px;border-radius:16px}li{margin:0 0 16px}audio{width:100%;height:32px;margin-top:6px}select,button{font:inherit;border:1px solid #8792a0;border-radius:8px;background:#202329;color:inherit;padding:8px}section{margin-top:40px}p{max-width:75ch}@media(max-width:900px){.mixes{grid-template-columns:1fr}}</style><main><h1>Which mix fits the starting song?</h1><p>Each player uses the same middle ten-second excerpt of that song. Listen before opening key.json. Rate each mix from 1 (poor fit) to 5 (strong fit). Model names are randomized separately for each seed. This worksheet is local and sends nothing.</p><p>These clips assess recommendation relevance; they do not test transition mixing or playback quality. No model is promoted from automatic scores alone.</p>'''+''.join(cards)+'''<p><button id="export">Export ratings</button></p></main><script>const controls=[...document.querySelectorAll('[data-rating]')];for(const control of controls){control.value=localStorage.getItem('musicos-blind-'+control.dataset.rating)||'';control.addEventListener('change',()=>localStorage.setItem('musicos-blind-'+control.dataset.rating,control.value));}document.getElementById('export').onclick=()=>{const data=Object.fromEntries(controls.map(control=>[control.dataset.rating,control.value]));const url=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='musicos-blind-ratings.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};</script></html>'''
(root/'review.html').write_text(page);os.chmod(root/'review.html',0o600);print('Private blind worksheet prepared; listening acceptance remains pending.')
