#!/usr/bin/env python3
"""Pin the packaging plugin; bridge static AppImage runtimes under Rosetta only.
The wrapper affects the builder, never the delivered application.
"""
from pathlib import Path
import hashlib,os,shlex,shutil,subprocess,urllib.request
URL='https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage'
SHA='49d6a17160675a6bd1781699aae6bdf7692d98552e02a3671d2183d10547842e'
def main():
 cache=Path(os.environ.get('XDG_CACHE_HOME',str(Path.home()/'.cache')))/'tauri'
 original=cache/'musicos-tools'/f'appimage-plugin-{SHA}.AppImage'
 original.parent.mkdir(parents=True,exist_ok=True)
 plugin=cache/'linuxdeploy-plugin-appimage.AppImage'
 if not original.exists():
  if plugin.exists() and hashlib.sha256(plugin.read_bytes()).hexdigest()==SHA:shutil.copyfile(plugin,original)
  else:
   with urllib.request.urlopen(URL,timeout=120) as response:original.write_bytes(response.read())
 if hashlib.sha256(original.read_bytes()).hexdigest()!=SHA:raise SystemExit('Packaging plugin changed upstream. Review it before updating the pinned checksum.')
 original.chmod(0o755)
 try:
  native=subprocess.run([str(original),'--plugin-api-version'],capture_output=True,timeout=30)
  if native.returncode:raise SystemExit(native.stderr.decode(errors='replace'))
 except OSError as error:
  if error.errno!=8:raise
  qemu=shutil.which('qemu-x86_64-static')
  if not qemu:raise SystemExit('This emulated builder needs qemu-user-static for the packaging runtime.')
  subprocess.run([qemu,str(original),'--appimage-extract-and-run','--plugin-api-version'],check=True,timeout=30)
  plugin.write_text('#!/bin/sh\n# musicOS builder-only static-runtime bridge\nexec '+shlex.quote(qemu)+' '+shlex.quote(str(original))+' --appimage-extract-and-run "$@"\n')
  plugin.chmod(0o755)
  print('Enabled packaging-runtime bridge for the emulated builder.')
 else:shutil.copyfile(original,plugin);plugin.chmod(0o755)
if __name__=='__main__':main()
