$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

if (-not $env:MUSICOS_COMPANION_PORT) {
    $env:MUSICOS_COMPANION_PORT = "18080"
}
if (-not $env:MUSICOS_MEDIA_DB) {
    $env:MUSICOS_MEDIA_DB = "out\desktop-media.sqlite3"
}
if (-not $env:MUSICOS_SYNC_TARGET) {
    $env:MUSICOS_SYNC_TARGET = "http://127.0.0.1:$($env:MUSICOS_COMPANION_PORT)"
}
if (-not $env:MUSICOS_SYNC_DIR) {
    $env:MUSICOS_SYNC_DIR = ".\music"
}

New-Item -ItemType Directory -Force -Path "out", $env:MUSICOS_SYNC_DIR | Out-Null

if (-not (Test-Path "node_modules\electron")) {
    npm.cmd install
}

npm.cmd run sync:electron
