#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

required_version="0.11.8"
installed_version="$(cargo packager --version 2>/dev/null | awk '{print $2}' || true)"
if [ "$installed_version" != "$required_version" ]; then
  echo "MusicOS packaging requires cargo-packager ${required_version}."
  echo "Install it with: cargo install cargo-packager --version ${required_version} --locked"
  exit 2
fi

case "$(uname -s)" in
  Darwin) formats="app,dmg" ;;
  Linux) formats="appimage" ;;
  *) echo "Use package-control-center.ps1 on Windows."; exit 2 ;;
esac

cargo packager --release -p musicos-desktop --formats "$formats"
