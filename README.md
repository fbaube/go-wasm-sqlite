# Go WASI SQLite Component

A small WebAssembly Component Model demonstration in which a TinyGo/WASI Preview 2 guest communicates with a native Rust host through WIT-defined interfaces.

The guest exposes application functionality and uses a database interface implemented by the host. The Rust host loads the resulting WebAssembly component with Wasmtime, supplies the imported database operations, and invokes the guest export.

## Architecture

```text
                 WIT contract
               wit/world.wit
                      |
        +-------------+-------------+
        |                           |
   Go/TinyGo guest              Rust host
   world: app                   world: host
        |                           |
        +------ WebAssembly -------+
                 component
```

### Guest

The Go guest is compiled with TinyGo for `wasip2`.

Its component world is:

```wit
world app {
    include wasi:cli/imports@0.2.0;
    import database;
    export application;
}
```

The WASI CLI imports are required by the TinyGo `wasip2` component build.

### Host

The native Rust host uses Wasmtime and generates bindings from the separate `host` world:

```wit
world host {
    import database;
    export application;
}
```

The two worlds share the same `database` and `application` interfaces, which are defined only once in `wit/world.wit`.

The generated Rust world binding is named `Host_`.

## Repository layout

```text
.
├── guest-go/
│   └── ...
├── host/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── scripts/
│   ├── build-go.sh
│   ├── check.sh
│   └── run-go-host.sh
├── wit/
│   ├── world.wit
│   └── deps/
│       └── ...
└── README.md
```

### Important files

- `wit/world.wit` — canonical WIT interfaces and worlds.
- `wit/deps/` — local WASI WIT dependencies.
- `guest-go/` — TinyGo component implementation.
- `host/src/main.rs` — native Wasmtime host.
- `scripts/build-go.sh` — builds and componentizes the Go guest.
- `scripts/run-go-host.sh` — runs the Rust host.
- `scripts/check.sh` — runs formatting, compilation, build, and smoke checks.

## Requirements

Install the following tools:

- Go.
- TinyGo with `wasip2` support.
- `wasm-tools`.
- Rust and Cargo.
- Wasmtime.
- SQLite development/runtime support as required by the host implementation.

Check the main tools:

```bash
go version
tinygo version
wasm-tools --version
rustc --version
cargo --version
wasmtime --version
```

The exact TinyGo, Rust, Wasmtime, and `wasm-tools` versions should match the versions used by the working development environment where possible.

## Build the guest

From the repository root:

```bash
./scripts/build-go.sh
```

The script compiles the Go guest and produces a WebAssembly component, normally under `target/`.

To locate generated WebAssembly files:

```bash
find . -type f -name '*.wasm' -print
```

The generated component can be inspected with:

```bash
wasm-tools component wit target/guest-go.wasm
```

Replace the path if `build-go.sh` writes the component elsewhere.

The component can also be validated with:

```bash
wasm-tools validate --features component-model target/guest-go.wasm
```

## Run the host

After building the guest:

```bash
./scripts/run-go-host.sh
```

The host:

1. Creates a Wasmtime engine and component linker.
2. Registers the required host-side imports.
3. Loads the generated Go component.
4. Instantiates the component.
5. Invokes the exported application.
6. Displays the result returned by the guest.

A successful run should include the expected database result, including:

```text
Ada
```

## Run all checks

Use the project check script after changes:

```bash
./scripts/check.sh
```

The script:

1. Verifies Rust formatting.
2. Runs `cargo check`.
3. Builds the Go component.
4. Runs the Rust host once.
5. Checks that the expected output contains `Ada`.

The script is intentionally a smoke test for the complete guest-to-host path.

The current form is:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE}")/.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo check --manifest-path host/Cargo.toml

./scripts/build-go.sh

output="$(./scripts/run-go-host.sh)"

printf '%s\n' "$output"

grep -F 'Ada' <<<"$output"
```

Run it from the repository root:

```bash
./scripts/check.sh
```

## WIT contract

The WIT package defines two application-specific interfaces.

The `database` interface provides database operations to the guest.

The `application` interface defines functionality exported by the guest for the host to invoke.

The package also defines two worlds:

- `app` — used by TinyGo when building the component.
- `host` — used by Rust/Wasmtime when generating host bindings.

Keep interface definitions in one place. Do not create duplicate declarations of `database` or `application` in additional `.wit` files within the same package.

## Development workflow

After making a change:

```bash
./scripts/check.sh
```

If the check passes, inspect the changes:

```bash
git status
git diff
```

Check for whitespace errors:

```bash
git diff --check
```

When the changes are ready:

```bash
git add <files>
git diff --cached
git commit -m "Describe the change"
```

Keep generated build output out of the repository unless it is intentionally part of the project.

## Troubleshooting

### `wasm-tools component new` cannot resolve a WASI import

If the error mentions an import such as:

```text
wasi:cli/environment@0.2.0
```

verify that `wit/world.wit` includes:

```wit
include wasi:cli/imports@0.2.0;
```

Also verify that the matching dependency exists under:

```text
wit/deps/wasi-cli-0.2.0/
```

### Duplicate WIT item

If `wasm-tools` reports:

```text
duplicate item named `database`
```

search all WIT files:

```bash
grep -RniE '^interface |^world ' wit --include='*.wit'
```

Each interface must be defined only once within the package.

### Rust cannot find `Host`

If Rust reports that `Host` does not exist but suggests `Host_`, use:

```rust
Host_::add_to_linker(...)
Host_::instantiate(...)
```

Do not import `crate::database::Host` for this purpose. That is the trait associated with the imported `database` interface, not the generated world binding.

### Formatting check reports changes

Apply formatting once:

```bash
cargo fmt --all
```

Review the resulting diff:

```bash
git diff --stat
git diff
```

Then rerun:

```bash
./scripts/check.sh
```

The check script uses `--check` so that it detects formatting changes without modifying files.

### The host runs twice

`check.sh` should capture the host output once:

```bash
output="$(./scripts/run-go-host.sh)"
```

Do not also invoke `./scripts/run-go-host.sh` on a separate line before that assignment.

## Design notes

This project deliberately keeps the Go build world and Rust binding world separate:

- TinyGo builds against `app`, which includes the WASI CLI imports required by the `wasip2` target.
- Rust host bindings target `host`, avoiding unnecessary duplicate WASI host bindings.
- Both worlds refer to the same application-specific interfaces.
- `wit/world.wit` remains the canonical contract.

This arrangement keeps the currently working TinyGo build intact while allowing the Rust host to expose only the application-specific imports it needs.

## Current status

The following workflow is working:

```bash
./scripts/build-go.sh
./scripts/run-go-host.sh
./scripts/check.sh
```

The repository has been checkpointed after confirming that the Go component builds, the Rust host compiles, the component instantiates, and the expected database output is produced.