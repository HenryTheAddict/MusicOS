$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$Image = if ($env:MUSICOS_BUILD_IMAGE) { $env:MUSICOS_BUILD_IMAGE } else { "musicos-buildroot:24.04" }
$OutVolume = if ($env:MUSICOS_BUILD_OUT_VOLUME) { $env:MUSICOS_BUILD_OUT_VOLUME } else { "musicos-buildroot-out" }
$BuildrootVersion = if ($env:BUILDROOT_VERSION) { $env:BUILDROOT_VERSION } else { "2026.05" }
$BuildJobs = if ($env:MUSICOS_BUILD_JOBS) { $env:MUSICOS_BUILD_JOBS } else { "1" }

docker info | Out-Null
docker build --platform linux/amd64 -f "$Root\docker\buildroot.Dockerfile" -t $Image "$Root"
docker run --rm `
    --platform linux/amd64 `
    -v "${Root}:/work" `
    -v "${OutVolume}:/work/out" `
    -v "${Root}\out:/host-out" `
    -w /work `
    -e "BUILDROOT_VERSION=$BuildrootVersion" `
    -e "MUSICOS_BUILD_JOBS=$BuildJobs" `
    $Image `
    bash -lc "set -euo pipefail; if [ -d out/rg35xx ] && [ ! -x out/rg35xx/host/bin/aarch64-buildroot-linux-musl-gcc ]; then rm -rf out/rg35xx; fi; rm -rf out/rg35xx/build/buildroot-config out/rg35xx/Makefile; scripts/build-rg35xx.sh; mkdir -p /host-out/rg35xx; rm -rf /host-out/rg35xx/images; cp -a /work/out/rg35xx/images /host-out/rg35xx/images; cp -f /work/out/rg35xx/.config /host-out/rg35xx/.config"
