#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
make -C src-desktop -j4
exec ./src-desktop/musicos-desktop "$@"
