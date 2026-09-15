#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

"$ROOT/scripts/build-go.sh"

GUEST_COMPONENT="$ROOT/target/guest-go.wasm" \
cargo run --manifest-path "$ROOT/host/Cargo.toml"
