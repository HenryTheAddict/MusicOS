#!/usr/bin/env python3
"""Retain exact published dependency notices and pinned repository license files."""
import json,subprocess,pathlib,shutil,urllib.request,urllib.error,concurrent.futures
root=pathlib.Path(__file__).resolve().parents[2]
metadata=json.loads(subprocess.check_output(['cargo','metadata','--format-version','1'],cwd=root,stderr=subprocess.DEVNULL));pkgs={p['id']:p for p in metadata['packages']};nodes={n['id']:n for n in metadata['resolve']['nodes']};start=next(k for k,p in pkgs.items() if p['name']=='musicos-visualizer');seen=set();stack=list(nodes[start]['dependencies'])
while stack:
 k=stack.pop()
 if k not in seen:seen.add(k);stack+=nodes[k]['dependencies']
fallbacks={'block':'MIT','objc_exception':'MIT','hexf-parse':'CC0-1.0','r-efi':'MIT'}
def copy(p):
 source=pathlib.Path(p['manifest_path']).parent;dest=root/'third_party/licenses/visualizer'/f"{p['name']}-{p['version']}";dest.mkdir(parents=True,exist_ok=True)
 files=[f for f in source.iterdir() if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE','UNLICENSE'))]
 if p.get('license_file'):
  f=source/p['license_file']
  if f not in files:files.append(f)
 for f in files:shutil.copyfile(f,dest/f.name)
 record={'name':p['name'],'version':p['version'],'license':p.get('license'),'repository':p.get('repository'),'files':[str(f.relative_to(root)) for f in dest.iterdir() if f.is_file()]}
 if not record['files']:
  vcs=source/'.cargo_vcs_info.json';info=json.loads(vcs.read_text()) if vcs.exists() else {};repo=(p.get('repository') or '').replace('http://','https://');parts=repo.split('/');slug='/'.join(parts[3:5]);revision=info.get('git',{}).get('sha1')
  candidates=[revision] if revision else [f"v{p['version']}",p['version'],'master']
  found=[]
  for rev in candidates:
   for name in ['LICENSE','LICENSE-MIT','LICENSE.MIT','LICENSE-APACHE','LICENSE.APACHE','LICENSE.md','LICENSE.txt','COPYING','COPYING.MIT','COPYING.APACHE']:
    url=f'https://raw.githubusercontent.com/{slug}/{rev}/{name}'
    try:
     with urllib.request.urlopen(url,timeout=10) as response:data=response.read(200000)
     if len(data)>100 and not data.startswith(b'<'):
      (dest/name).write_bytes(data);found.append({'path':str((dest/name).relative_to(root)),'source':url})
    except (urllib.error.URLError,TimeoutError):pass
   if found:break
  record['sources']=found;record['files']=[f['path'] for f in found]
 if p['name'] in fallbacks and not files:
  selected=fallbacks[p['name']]
  # These releases declare SPDX alternatives but omit separate license files.
  # Retain the published manifest verbatim, plus the full chosen license text.
  shutil.copyfile(source/'Cargo.toml',dest/'UPSTREAM-MANIFEST.toml')
  url=f'https://raw.githubusercontent.com/spdx/license-list-data/v3.27.0/text/{selected}.txt'
  target=dest/f'LICENSE-{selected}.txt'
  if not target.exists():
   with urllib.request.urlopen(url,timeout=15) as response:license_text=response.read(200000).decode()
   if selected=='MIT':license_text=license_text.replace('Copyright (c) <year> <copyright holders>','Copyright and author information: see the original published manifest and NOTICE.txt distributed alongside this license.')
   target.write_text(license_text)
  (dest/'NOTICE.txt').write_text(f"{p['name']} {p['version']} declares {p.get('license')} in its published manifest. This distribution selects {selected}. The upstream archive did not include a separate license text; the complete SPDX license text is retained here alongside its unmodified manifest. Authors as listed upstream: {', '.join(p.get('authors',[]))}. Repository: {p.get('repository')}.\n")
  record['selected_license']=selected
  record['sources']=[{'path':str(target.relative_to(root)),'source':url}]
  record['files']=[str(f.relative_to(root)) for f in sorted(dest.iterdir()) if f.is_file()]
 return record
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:records=list(pool.map(copy,[pkgs[k] for k in sorted(seen)]))
(root/'assets/visualizers/renderer-dependencies.json').write_text(json.dumps(records,indent=2)+'\n')
missing=[r['name'] for r in records if not r['files']]
print(f'{len(records)} dependencies; missing notices: {missing}')
if missing:raise SystemExit(1)
