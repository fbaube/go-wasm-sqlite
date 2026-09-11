use std::path::PathBuf;

use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Config, Engine, Store,
};
use wasmtime_wasi::{
    WasiCtx,
    WasiCtxBuilder,
    WasiCtxView,
    WasiView,
};

use crate::demo::db::database;

wasmtime::component::bindgen!({
    world: "app",
    path: "../wit",
});

struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
    database: rusqlite::Connection,
}

impl wasmtime::component::HasData for HostState {
    type Data<'a> = &'a mut HostState;
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl database::Host for HostState {
    fn execute(&mut self, sql: String) -> Result<u64, String> {
        self.database
            .execute_batch(&sql)
            .map(|_| 0)
            .map_err(|e| e.to_string())
    }

    fn query(
        &mut self,
        sql: String,
    ) -> Result<database::QueryResult, String> {
        let mut statement = self.database
            .prepare(&sql)
            .map_err(|e| e.to_string())?;

        let columns = statement
            .column_names()
            .iter()
            .map(|name| name.to_string())
            .collect();

        let column_count = statement.column_count();

        let rows = statement
            .query_map([], |row| {
                let mut values = Vec::with_capacity(column_count);

                for index in 0..column_count {
                    let value: rusqlite::types::Value = row
                        .get(index)?;

                    let text = match value {
                        rusqlite::types::Value::Null => "NULL".to_string(),
                        rusqlite::types::Value::Integer(value) => value.to_string(),
                        rusqlite::types::Value::Real(value) => value.to_string(),
                        rusqlite::types::Value::Text(value) => value,
                        rusqlite::types::Value::Blob(value) => {
                            format!("<blob {} bytes>", value.len())
                        }
                    };

                    values.push(text);
                }

                Ok(values)
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(database::QueryResult { columns, rows })
    }
}

fn main() -> anyhow::Result<()> {
    let config = Config::new();

    let engine = Engine::new(&config)
        .map_err(|e| anyhow::anyhow!("failed to create Wasmtime engine: {e}"))?;

    let component_path = std::env::var_os("GUEST_COMPONENT")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/wasm32-wasip2/release/guest_simple.wasm")
    });

    println!(
        "loading guest component: {}",
        component_path.display()
    );

    let component = Component::from_file(&engine, &component_path)
        .map_err(|e| anyhow::anyhow!("failed to load guest component: {e}"))?;

    let mut linker: Linker<HostState> = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| anyhow::anyhow!("failed to add WASI imports: {e}"))?;

    let database = rusqlite::Connection::open("demo.sqlite")
        .map_err(|e| anyhow::anyhow!("failed to open SQLite database: {e}"))?;

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build();

    let mut store = Store::new(
        &engine,
        HostState {
            wasi,
            table: ResourceTable::new(),
            database,
        },
    );

App::add_to_linker::<HostState, HostState>(
    &mut linker,
    |state: &mut HostState| state,
)
.map_err(|e| anyhow::anyhow!("failed to add database interface: {e}"))?;


    let bindings = App::instantiate(&mut store, &component, &linker)
        .map_err(|e| anyhow::anyhow!("failed to instantiate guest: {e}"))?;

    let result = bindings
        .demo_db_application()
        .call_run(&mut store)
        .map_err(|e| anyhow::anyhow!("guest application.run failed: {e}"))?;

    println!("{result}");

    Ok(())
}
