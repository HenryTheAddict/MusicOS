$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

cargo check --workspace --features desktop-app
cargo test --workspace --features desktop-app
