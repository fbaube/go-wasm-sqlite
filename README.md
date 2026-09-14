# Go WASI SQLite Component

A proof of concept demonstrating a Go WebAssembly Component calling SQLite through a Rust host.

The Go guest is compiled with TinyGo to a WASI Preview 2 component. It imports a shared WIT database interface, while the Rust host implements that interface using `rusqlite`.

```text
Rust host
 provides WASI Preview 2 imports  
 implements demo:db/database  
 execute(sql)       
 query(sql)       
 opens SQLite with rusqlite  
 invokes demo:db/application.run  

Go component
 imports demo:db/database  
 exports demo:db/application.run  
```

## Current status

The end-to-end path works:

1. The Go component creates a SQLite table.
2. It inserts a row.
3. It queries the table.
4. The Rust host executes the SQL.
5. The query result crosses the WIT boundary back into Go.
6. Go formats and returns the result.

Example output:

```text
id | name
----------------
1 | Ada
```

The current host uses a persistent SQLite database named:

```text
demo.sqlite
```

## Project layout

The relevant files are organized approximately as follows:

```text
go-wasm-sqlite/
 host/
 Cargo.toml   
 src/   
 main.rs       
 guest-go/
 go.mod   
 main.go   
 gen/   
 demo/      
 wasi/      
 wit/   
 world.wit       
 deps/       
 guest-simple/
 ...   
 wit/
 ...   
 target/
 guest-go.wasm    
```

The exact layout may evolve. The important boundary is the shared WIT package, not the source language.

## Software versions

The following versions were used successfully.

### Go component toolchain

```text
Go       go1.27.1 darwin/arm64
TinyGo   0.42.0 darwin/arm64
LLVM     22.1.4
```

### WIT and WebAssembly tools

```text
wit-bindgen CLI   0.61.1
wkg                0.10.0
```

Record these versions locally with:

```bash
wasm-tools --version
wasmtime --version
```

### Rust host

The Rust host was built with:

```text
wasmtime         48.0.1
wasmtime-wasi    48.0.1
rusqlite         0.32.1
```

Also record:

```bash
rustc --version
cargo --version
```

### Go runtime dependencies

The generated Go bindings use the Bytecode Alliance Go Component Model runtime. The working generated imports use:

```go
github.com/bytecodealliance/wasm-tools-go/cm
```

The generated bindings also use the runtime package from:

```text
go.bytecodealliance.org/pkg
```

Inspect the exact versions with:

```bash
grep -nE \
  'wasm-tools-go|go.bytecodealliance.org' \
  guest-go/go.mod
```

### Why versions matter

This experiment crossed several rapidly changing tool boundaries:

- TinyGo WASI Preview 2 component support;
- WIT binding generation;
- the Go Component Model runtime;
- wasm-tools;
- Wasmtime component bindings;
- WASI Preview 2 package versions.

Small version differences caused significant problems, particularly around generated Go imports and runtime package namespaces. Reproducing the working setup requires recording compiler, generator, runtime, and host versions.

## Shared WIT interface

The core interface is intentionally small:

```wit
package demo:db;

interface database {
    record query-result {
        columns: list<string>,
        rows: list<list<string>>,
    }

    execute: func(sql: string) -> result<u64, string>;
    query: func(sql: string) -> result<query-result, string>;
}

interface application {
    run: func() -> string;
}

world app {
    include wasi:cli/imports@0.2.0;
    import database;
    export application;
}
```

The `database` interface is the capability boundary. The Go guest does not open SQLite directly. It receives database functionality from the host through the component import.

The `application` interface is the guest entry point. The Rust host calls:

```text
demo:db/application.run
```

## Building the Go component

From the project root:

```bash
cd guest-go
```

Fetch WIT dependencies:

```bash
wkg wit fetch
```

Build the component:

```bash
mkdir -p ../target

tinygo build \
  -target wasip2 \
  -wit-package ./wit \
  -wit-world app \
  -o ../target/guest-go.wasm \
  .
```

Inspect the component:

```bash
wasm-tools component wit ../target/guest-go.wasm
```

The component should export:

```text
demo:db/application
```

and, because it uses the database calls, import:

```text
demo:db/database
```

## Running through the Rust host

From the project root:

```bash
GUEST_COMPONENT="$PWD/target/guest-go.wasm" \
cargo run --manifest-path host/Cargo.toml
```

The `GUEST_COMPONENT` environment variable is important. Without it, the host may load its default Rust guest component instead of the Go component.

The host:

1. creates a Wasmtime engine;
2. configures WASI;
3. opens `demo.sqlite`;
4. registers the `demo:db/database` implementation;
5. loads the component specified by `GUEST_COMPONENT`;
6. instantiates the component;
7. invokes `demo:db/application.run`.

## Binding generation

The Go bindings were generated with the Bytecode Alliance Go generator:

```bash
wit-bindgen-go generate \
  --world app \
  --out gen \
  ./wit
```

Depending on installation, the command may be invoked through:

```bash
"$(go env GOPATH)/bin/wit-bindgen-go" generate \
  --world app \
  --out gen \
  ./wit
```

Generated packages include:

```text
gen/demo/db/application
gen/demo/db/database
gen/wasi/...
```

The application export is registered through the generated package:

```go
application.Exports.Run = func() string {
    // guest implementation
}
```

The assignment must occur in `init()`:

```go
func init() {
    application.Exports.Run = func() string {
        return "Go component started"
    }
}
```

Using `main()` alone caused a nil-pointer trap when Wasmtime invoked the exported component function. Component exports can be called before ordinary program flow reaches `main()`, so registration belongs in initialization.

## Go implementation

The current implementation calls the generated database bindings:

```go
create := database.Execute(
    "create table if not exists people (id integer primary key, name text)",
)

insert := database.Execute(
    "insert or ignore into people (id, name) values (1, 'Ada')",
)

query := database.Query(
    "select id, name from people order by id",
)
```

The generated bindings represent WIT results with:

```go
cm.Result[Shape, OK, Err]
```

They are not Go's conventional:

```go
(value, error)
```

form.

The result must be inspected using:

```go
if result.IsErr() {
    return *result.Err()
}

value := result.OK()
```

The generated `list<T>` values are also not ordinary Go slices. They are represented by:

```go
cm.List[T]
```

To iterate over them, convert them to a Go slice view:

```go
for _, column := range result.Columns.Slice() {
    // ...
}

for _, row := range result.Rows.Slice() {
    for _, value := range row.Slice() {
        // ...
    }
}
```

`Slice()` does not copy the underlying data; it exposes the list storage as a Go slice view.

## Technical constraints overcome

### Component exports are not ordinary Go exports

A raw Go or WebAssembly export is not automatically a WIT interface export. Generated bindings must connect:

```text
demo:db/application.run
```

to the Go implementation.

Using the generated `application.Exports.Run` table solved this.

### Initialization timing

Initially, the Go component built but trapped with a nil-pointer dereference when invoked. The generated adapter called an unset function pointer.

The fix was:

```go
func init() {
    application.Exports.Run = ...
}
```

rather than assigning the callback only inside `main()`.

### WASI CLI imports

TinyGo's `wasip2` pipeline expected WASI CLI imports, including:

```text
wasi:cli/environment@0.2.0
```

The custom WIT world therefore had to include:

```wit
include wasi:cli/imports@0.2.0;
```

The corresponding WASI packages were fetched into:

```text
wit/deps/
```

with:

```bash
wkg wit fetch
```

### Generated Go package layout

The generated Go source is a tree of packages, not one package at the top-level `gen` directory:

```text
gen/demo/db/application
gen/demo/db/database
gen/wasi/...
```

Therefore this is not a valid build target:

```bash
tinygo build ./gen
```

The root `main` package must import the generated package it uses.

### Component-model result values

The database interface uses:

```wit
result<u64, string>
```

The Go binding represents that as a typed `cm.Result`, requiring explicit success/error inspection.

### Component-model lists

The query result uses:

```wit
columns: list<string>,
rows: list<list<string>>,
```

The Go runtime represents those as `cm.List` values with methods such as:

```go
Len() uintptr
Slice() []T
```

They cannot be ranged over directly.

### Rust host registration

The Rust host uses Wasmtime's generated component bindings:

```rust
wasmtime::component::bindgen!({
    world: "app",
    path: "../wit",
});
```

The host implements:

```rust
impl database::Host for HostState
```

and registers the generated interface with:

```rust
App::add_to_linker::<HostState, HostState>(
    &mut linker,
    |state: &mut HostState| state,
)?;
```

The Go guest and Rust host must use the same WIT package identity and definitions. Similar-looking interfaces generated from different packages or worlds are not interchangeable.

### Runtime package namespace mismatch

The newer Go generator initially emitted imports using:

```go
go.bytecodealliance.org/cm
```

The available published module metadata was inconsistent with that path. The working generated files used:

```go
github.com/bytecodealliance/wasm-tools-go/cm
```

and the project added:

```bash
go get github.com/bytecodealliance/wasm-tools-go@v0.3.2
go mod tidy
```

This is a fragile area and should be revisited when upgrading the generator or TinyGo.

## Roadmap: database/sql

The current Go guest calls the generated WIT functions directly:

```go
database.Execute(sql)
database.Query(sql)
```

The next goal is to expose the host database through Go's standard `database/sql` API.

### Goal

The desired guest-side code would look like:

```go
db, err := sql.Open("componentdb", "")
if err != nil {
    return err
}
defer db.Close()

_, err = db.Exec(
    "create table if not exists people (id integer primary key, name text)",
)
if err != nil {
    return err
}

rows, err := db.Query("select id, name from people")
if err != nil {
    return err
}
defer rows.Close()
```

The WIT boundary would remain the same initially or evolve into a more structured database API.

### Phase 1: create a small driver

Implement a Go `database/sql/driver.Driver`:

```go
type Driver struct{}

func (Driver) Open(name string) (driver.Conn, error) {
    return &Conn{}, nil
}
```

Register it:

```go
sql.Register("componentdb", Driver{})
```

Then implement:

```go
type Conn struct{}
type Stmt struct{}
type Rows struct{}
```

The first version can support only:

- `Exec`;
- `Query`;
- no transactions;
- no prepared statements;
- string-only values;
- one connection.

This provides the standard Go API without requiring a full database engine inside the component.

### Phase 2: map Exec to WIT

Implement:

```go
func (c *Conn) ExecContext(
    ctx context.Context,
    query string,
    args []driver.NamedValue,
) (driver.Result, error)
```

Initially reject arguments:

```go
if len(args) != 0 {
    return nil, errors.New("parameters not supported yet")
}
```

Call:

```go
result := database.Execute(query)
```

Map the returned `u64` to:

```go
driver.Result
```

A simple result implementation could report:

```go
type Result struct {
    affected uint64
}
```

### Phase 3: map Query to Rows

Implement:

```go
func (c *Conn) QueryContext(
    ctx context.Context,
    query string,
    args []driver.NamedValue,
) (driver.Rows, error)
```

Call:

```go
result := database.Query(query)
```

Then adapt:

```go
cm.List[string]
cm.List[cm.List[string]]
```

to the `database/sql/driver.Rows` methods:

```go
Columns() []string
Next(dest []driver.Value) error
Close() error
```

The first implementation can return every cell as a string.

### Phase 4: add parameters

The existing WIT interface accepts only SQL text:

```wit
execute: func(sql: string) -> result<u64, string>;
query: func(sql: string) -> result<query-result, string>;
```

That is sufficient for the initial proof of concept but not ideal for production.

A future interface could use:

```wit
variant value {
    null,
    integer(s64),
    float64(float64),
    text(string),
    blob(list<u8>),
}
```

and:

```wit
execute: func(
    sql: string,
    params: list<value>,
) -> result<u64, string>;
```

The same parameter type could be used for `query`.

### Phase 5: transactions

Add a transaction abstraction, either through explicit WIT functions:

```wit
begin: func() -> result<transaction-id, string>;
commit: func(transaction-id) -> result<_, string>;
rollback: func(transaction-id) -> result<_, string>;
```

or by introducing a WIT resource:

```wit
resource transaction {
    execute: func(sql: string) -> result<u64, string>;
    query: func(sql: string) -> result<query-result, string>;
    commit: func() -> result<_, string>;
    rollback: func() -> result<_, string>;
}
```

The Go `database/sql` driver would then implement:

```go
BeginTx
```

and return a `driver.Tx`.

### Phase 6: prepared statements

A production-quality `database/sql` driver should support:

```go
PrepareContext
```

This requires either:

- adding prepared-statement functions to WIT; or
- emulating prepared statements by retaining SQL text and binding parameters during each call.

The latter is easier to prototype but does not provide true SQLite prepared-statement behavior.

### Phase 7: context cancellation

The current synchronous WIT functions do not accept cancellation:

```wit
execute: func(...)
query: func(...)
```

A future design should consider:

- WASI Preview 2 async support;
- explicit cancellation handles;
- host-side timeouts;
- keeping database operations short and bounded.

Until then, `database/sql` context cancellation can only be advisory.

## Proposed WIT evolution

The current interface is intentionally simple. A more complete version might become:

```wit
package demo:db;

variant value {
    null,
    integer(s64),
    float64(float64),
    text(string),
    blob(list<u8>),
}

record query-result {
    columns: list<string>,
    rows: list<list<value>>,
}

interface database {
    execute: func(
        sql: string,
        params: list<value>,
    ) -> result<u64, string>;

    query: func(
        sql: string,
        params: list<value>,
    ) -> result<query-result, string>;
}

interface application {
    run: func() -> string;
}

world app {
    include wasi:cli/imports@0.2.0;
    import database;
    export application;
}
```

Do not adopt this expanded interface until the direct `database/sql` adapter has been tested against the current string-only interface.

## Planned CLI interface

After the README and `database/sql` design are stable, the next feature is a CLI interface for the Go component.

A likely first version would expose a Go guest function such as:

```text
run(sql: string) -> string
```

or:

```text
run(args: list<string>) -> string
```

The CLI could then:

1. parse a SQL statement or command;
2. invoke the database interface;
3. format tabular results;
4. return or print the result;
5. eventually support commands such as `.tables`, `.schema`, and `.quit`.

There are two possible designs.

### Host-driven CLI

The Rust host remains responsible for process arguments and standard input. It passes commands into the Go component through an exported function.

Advantages:

- simpler component boundary;
- easier testing;
- database access remains in one host;
- no reliance on WASI argument/stdin behavior inside the guest.

### Guest-driven CLI

The Go component uses WASI CLI interfaces directly:

- arguments;
- stdin;
- stdout;
- stderr.

Advantages:

- more conventional Go CLI structure;
- the component can run as a standalone CLI.

Disadvantages:

- more WASI bindings;
- more behavior to test across Wasmtime and TinyGo;
- less convenient when embedding the component from Rust.

The recommended next step is the host-driven CLI. It preserves the current successful architecture and lets the Go component focus on parsing and formatting rather than process management.

## Reproducibility checklist

Before upgrading anything, record:

```bash
go version
tinygo version
wit-bindgen --version
wkg --version
wasm-tools --version
wasmtime --version
rustc --version
cargo --version
```

Also record:

```bash
grep -nE \
  'wasmtime|wasmtime-wasi|rusqlite|wasm-tools-go|go.bytecodealliance' \
  host/Cargo.toml \
  guest-go/go.mod
```

Keep the following artifacts together:

```text
wit/
wit/deps/
guest-go/gen/
guest-go/go.mod
guest-go/go.sum
host/Cargo.lock
```

Generated bindings should be regenerated only when deliberately upgrading the WIT or generator toolchain.
