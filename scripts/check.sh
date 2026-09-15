#!/usr/bin/env bash
set -euo pipefail

# "This is the first command to run after every change."

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo check --manifest-path host/Cargo.toml

./scripts/build-go.sh

output="$(./scripts/run-go-host.sh)"

printf '%s\n' "$output"

grep -F 'Ada' <<<"$output"
