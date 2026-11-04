#!/usr/bin/env python3
"""Stage the Linux CPU runtime from the checksum-pinned official PyPI wheel.
Never touch Mac resources or use a library selected by model contents.
"""
from pathlib import Path
import hashlib,io,json,platform,sys,urllib.request,zipfile
ROOT=Path(__file__).resolve().parents[2]
URL='https://files.pythonhosted.org/packages/a6/72/6c57163b63b5343853d7f0619c4f424a6e53ee762d7263667ff004bfede1/onnxruntime-1.31.0-cp311-cp311-manylinux_2_28_x86_64.whl'
SHA256='d4092b78fc5bab77ce6522393098cdb2535423045ecdcff15cc0d022162d6b66'
def main():
 if sys.platform!='linux' or platform.machine()!='x86_64':raise SystemExit('Stage this runtime only in an Intel/AMD Linux build environment.')
 folder=ROOT/'src-tauri/resources/runtime';folder.mkdir(parents=True,exist_ok=True)
 cache=ROOT/'out/linux-runtime';cache.mkdir(parents=True,exist_ok=True);wheel=cache/'onnxruntime-1.31.0-linux-x86_64.whl'
 if not wheel.exists():
  with urllib.request.urlopen(URL,timeout=120) as response:wheel.write_bytes(response.read())
 data=wheel.read_bytes();assert hashlib.sha256(data).hexdigest()==SHA256,'Linux runtime archive checksum mismatch'
 files={}
 with zipfile.ZipFile(io.BytesIO(data)) as archive:
  for name in archive.namelist():
   base=Path(name).name
   if base in ['libonnxruntime.so.1.31.0','libonnxruntime_providers_shared.so']:
    content=archive.read(name);(folder/base).write_bytes(content);files[base]=hashlib.sha256(content).hexdigest()
   if name in ['onnxruntime/LICENSE','onnxruntime/ThirdPartyNotices.txt']:
    dest=ROOT/'third_party/licenses/onnxruntime-linux';dest.mkdir(parents=True,exist_ok=True);(dest/base).write_bytes(archive.read(name))
 assert len(files)==2,'Expected both CPU runtime libraries'
 (cache/'runtime-manifest.json').write_text(json.dumps({'version':'1.31.0','source':URL,'archive_sha256':SHA256,'files':files},indent=2)+'\n')
 print('Verified Linux CPU runtime:',', '.join(files))
if __name__=='__main__':main()
