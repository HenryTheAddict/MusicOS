$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

cargo check --workspace --features desktop-app,desktop-audio
cargo test --workspace --features desktop-app,desktop-audio
