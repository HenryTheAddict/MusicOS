#!/usr/bin/env bash
# Run inside scripts/linux/Dockerfile's amd64 Ubuntu 22.04 environment.
set -euo pipefail
cd "$(dirname "$0")/../.."
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'Build this package in an Intel/AMD Linux environment.' >&2; exit 1; }
export APPIMAGE_EXTRACT_AND_RUN=1
python3 scripts/linux/prepare-runtime.py
python3 scripts/linux/prepare-appimage-tools.py
python3 - <<'PY'
from pathlib import Path
import json,hashlib
root=Path('src-tauri/resources/models/clap-general-v1')
manifest=json.loads((root/'manifest.json').read_text())
for item in manifest['files']:
 data=(root/item['name']).read_bytes()
 assert len(data)==item['bytes'] and hashlib.sha256(data).hexdigest()==item['sha256'],f"Model mismatch: {item['name']}"
print('Bundled model files verified; no model replacement or processing.')
PY
npm ci
npm run tauri -- build --bundles appimage --config '{"build":{"beforeBuildCommand":"npm run build:ui && node scripts/prepare-desktop.mjs"}}'
python3 - <<'PY'
from pathlib import Path
import json,hashlib,subprocess
files=list(Path('target/release/bundle/appimage').glob('*.AppImage'))
assert files,'AppImage was not produced'
report={'platform':'x86_64-unknown-linux-gnu','baseline':'Ubuntu 22.04 / glibc 2.35','rust':subprocess.check_output(['rustc','--version'],text=True).strip(),'runtime':json.loads(Path('out/linux-runtime/runtime-manifest.json').read_text()),'artifacts':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in files}}
Path('out/linux-runtime/build-manifest.json').write_text(json.dumps(report,indent=2)+'\n')
for name,sha in report['artifacts'].items():print(sha,name)
PY
