cargo build -p wasi_sql_demo_host
argo build --target wasm32-wasip2  -p guest_simple   --release

