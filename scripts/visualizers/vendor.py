#!/usr/bin/env python3
"""Reproduce the curated, pinned visualizer sources; never downloads at app runtime."""
from pathlib import Path
import urllib.request,urllib.parse,hashlib,json
root=Path(__file__).resolve().parents[2]
revision='c10e2616762f262de898e5d4c1f162a3e01340c1'
rows=[('boxes','suksma - pus on my cat - j4+ - cubby'),('oil-puddle','undulate harque - shit broo'),('carbonated','Zylot - In death there is life (Geiss Layered Mix) (Jelly)'),('watercolor','$$$ Royal - Mashup (441)'),('cells','$$$ Royal - Mashup (431)'),('tweaker','$$$ Royal - Mashup (337)'),('universe','$$$ Royal - Mashup (307)'),('mold','flexi - borderline imagery'),('bazinga','flexi - bouncing balls [double mindblob gastrointestinal mix]'),('infinite','Flexi, fishbrain + Martin - witchery'),('outfinate','Flexi, fishbrain, Geiss + Martin - tokamak witchery'),('paint-stir','Flexi, martin + geiss - dedicated to the sherwin maxawow'),('mucus','martin - mucus cervix')]
entries=[]
for id,name in rows:
 original='presets/full/'+name+'.json';url='https://raw.githubusercontent.com/jberg/butterchurn-presets/'+revision+'/'+urllib.parse.quote(original)
 data=urllib.request.urlopen(url).read();json.loads(data);path=root/'assets/visualizers/original'/f'{id}.json';path.write_bytes(data)
 entries.append(dict(id=id,name='Universe' if id=='universe' else id.replace('-',' '),original_filename=name+'.json',authors={'boxes':'suksma, j4+, cubby','carbonated':'Zylot; Geiss (layered mix)'}.get(id,name.split(' - ')[0]),source_url='https://github.com/jberg/butterchurn-presets/blob/'+revision+'/'+urllib.parse.quote(original),sha256=hashlib.sha256(data).hexdigest(),source='original/'+id+'.json',license='MIT',port=dict(equations='crates/musicos-visualizer/src/eel.rs',pixel_equations='crates/musicos-visualizer/src/pixel.rs',shaders='crates/musicos-visualizer/src/shaders.rs',drawing='crates/musicos-visualizer/src/sprites.rs',feedback='crates/musicos-visualizer/src/renderer.rs'),modifications=['Original JSON retained byte-for-byte; Rust equation and GPU ports recorded separately.']))
(root/'assets/visualizers/manifest.json').write_text(json.dumps(dict(version=1,presets_revision=revision,engine_reference_revision='fbac2f6bab62fd9c6a50ebbeb29359c5eb05903e',presets=entries),indent=2)+'\n')
for repo in ['butterchurn','butterchurn-presets']:
 rev='fbac2f6bab62fd9c6a50ebbeb29359c5eb05903e' if repo=='butterchurn' else revision
 (root/'third_party/licenses/butterchurn'/f'{repo}-MIT.txt').write_bytes(urllib.request.urlopen(f'https://raw.githubusercontent.com/jberg/{repo}/{rev}/LICENSE').read())
print('Vendored exactly',len(entries),'presets with provenance and licenses.')
