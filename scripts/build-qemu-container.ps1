$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$Image = if ($env:MUSICOS_BUILD_IMAGE) { $env:MUSICOS_BUILD_IMAGE } else { "musicos-buildroot:24.04" }
$OutVolume = if ($env:MUSICOS_BUILD_OUT_VOLUME) { $env:MUSICOS_BUILD_OUT_VOLUME } else { "musicos-buildroot-out" }
$BuildrootVersion = if ($env:BUILDROOT_VERSION) { $env:BUILDROOT_VERSION } else { "2026.05" }

docker info | Out-Null
docker build --platform linux/amd64 -f "$Root\docker\buildroot.Dockerfile" -t $Image "$Root"
docker run --rm `
    --platform linux/amd64 `
    -v "${Root}:/work" `
    -v "${OutVolume}:/work/out" `
    -v "${Root}\out:/host-out" `
    -w /work `
    -e "BUILDROOT_VERSION=$BuildrootVersion" `
    $Image `
    bash -lc "set -euo pipefail; rm -rf out/qemu/build/buildroot-config out/qemu/Makefile; scripts/build-qemu.sh; mkdir -p /host-out/qemu; rm -rf /host-out/qemu/images; cp -a /work/out/qemu/images /host-out/qemu/images; cp -f /work/out/qemu/.config /host-out/qemu/.config"
