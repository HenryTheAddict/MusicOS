$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

if (-not (Test-Path "node_modules\electron")) {
    npm.cmd install
}

cargo build --bin musicos-mac --bin musicos-sync --bin musicos-desktop --features desktop-app,desktop-audio
npm.cmd exec electron -- --version | Out-Null

Write-Host "Built MusicOS desktop tools in target\debug."
