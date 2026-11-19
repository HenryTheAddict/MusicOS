"""Stage the checksum-pinned Windows x64 ONNX CPU runtime and notices."""

import hashlib
import io
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
URL = (
    "https://files.pythonhosted.org/packages/37/de/"
    "6cab7e39917cc87728d2f00abe97c81fe86b29f9e1f758627864c28f0c21/"
    "onnxruntime-1.31.0-cp311-cp311-win_amd64.whl"
)
SHA256 = "317608967b03807ed4661113b08293fac02a1db6496a6863a07d9f19232936ad"


def main():
    with urllib.request.urlopen(URL, timeout=120) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise SystemExit("Windows runtime archive checksum mismatch")
    runtime = ROOT / "src-tauri/resources/runtime"
    licenses = ROOT / "third_party/licenses/onnxruntime-windows"
    for folder in (runtime, licenses):
        folder.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        for name in ("onnxruntime.dll", "onnxruntime_providers_shared.dll"):
            (runtime / name).write_bytes(archive.read("onnxruntime/capi/" + name))
        for name in ("LICENSE", "ThirdPartyNotices.txt"):
            (licenses / name).write_bytes(archive.read("onnxruntime/" + name))


if __name__ == "__main__":
    main()
