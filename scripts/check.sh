#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo check --manifest-path host/Cargo.toml

./scripts/build-go.sh
./scripts/run-go-host.sh
