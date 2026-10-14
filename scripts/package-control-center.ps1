$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$RequiredVersion = "0.11.8"
$InstalledVersion = ""
try {
    $InstalledVersion = ((cargo packager --version) -split " ")[1]
} catch {
    $InstalledVersion = ""
}

if ($InstalledVersion -ne $RequiredVersion) {
    Write-Host "MusicOS packaging requires cargo-packager $RequiredVersion."
    Write-Host "Install it with: cargo install cargo-packager --version $RequiredVersion --locked"
    exit 2
}

cargo packager --release -p musicos-desktop --formats "nsis"
