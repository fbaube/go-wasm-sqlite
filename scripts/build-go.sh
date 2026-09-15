#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

GEN="$ROOT/guest-go/gen"

rm -rf "$GEN"

(
    cd "$ROOT/guest-go"

    "$(go env GOPATH)/bin/wit-bindgen-go" generate \
        --world app \
        --out gen \
        ../wit
)

grep -RIl 'go.bytecodealliance.org/cm' "$GEN" \
    | xargs -r sed -i '' \
        's#go\.bytecodealliance\.org/cm#github.com/bytecodealliance/wasm-tools-go/cm#g'

mkdir -p "$ROOT/target"

(
    cd "$ROOT/guest-go"

    tinygo build \
        -target wasip2 \
        -wit-package ../wit \
        -wit-world app \
        -o "$ROOT/target/guest-go.wasm" \
        .
)
