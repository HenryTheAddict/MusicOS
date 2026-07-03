#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/env.sh"

cargo fmt --check
cargo check --workspace --features desktop-app,desktop-audio
cargo test --workspace --features desktop-app,desktop-audio
