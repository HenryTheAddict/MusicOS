"""Fetch official comparison weights with bounded HTTP ranges and LFS verification.
A cache-only helper for hosts where the optional Xet client stalls. No app writes.
"""
import argparse,hashlib,json,os,urllib.request
from pathlib import Path
from huggingface_hub import HfApi
p=argparse.ArgumentParser();p.add_argument('repository');p.add_argument('output',type=Path);p.add_argument('--metadata-only',action='store_true');a=p.parse_args()
info=HfApi().model_info(a.repository,files_metadata=True,token=False);root=a.output/('models--'+a.repository.replace('/','--'));root.mkdir(parents=True,exist_ok=True)
(root/'pinned.json').write_text(json.dumps({'repository':a.repository,'revision':info.sha},indent=2))
for item in info.siblings:
 if not item.rfilename.endswith(('.bin','.safetensors','.json','.py','.txt','.model','.md')):continue
 if a.metadata_only and item.rfilename.endswith(('.bin','.safetensors')):continue
 path=root/item.rfilename;path.parent.mkdir(parents=True,exist_ok=True)
 expected=item.lfs.sha256 if item.lfs else None
 if path.exists() and path.stat().st_size==item.size and (not expected or hashlib.file_digest(open(path,'rb'),'sha256').hexdigest()==expected):continue
 partial=path.with_suffix(path.suffix+'.partial');size=item.size;start=partial.stat().st_size if partial.exists() else 0
 with partial.open('ab') as output:
  while start<size:
   end=min(size-1,start+16*1024*1024-1);url=f'https://huggingface.co/{a.repository}/resolve/{info.sha}/{item.rfilename}?download=true&range_start={start}'
   request=urllib.request.Request(url,headers={'Range':f'bytes={start}-{end}','Accept-Encoding':'identity'})
   with urllib.request.urlopen(request,timeout=45) as response:
    content=response.read(end-start+2)
    if response.status==206:assert response.headers.get('Content-Range','').startswith(f'bytes {start}-'), 'Wrong range returned'
    else:assert start==0 and response.status==200 and size<=len(content),'Server did not honor bounded download'
   assert len(content)==end-start+1,'Wrong download length';output.write(content);output.flush();start+=len(content)
   if size>32*1024*1024:print(f'{a.repository}: {start//1048576}/{size//1048576} MiB',flush=True)
 if expected:assert hashlib.file_digest(open(partial,'rb'),'sha256').hexdigest()==expected,'LFS checksum mismatch'
 partial.replace(path)
print(f'Verified official snapshot {info.sha}',flush=True)
