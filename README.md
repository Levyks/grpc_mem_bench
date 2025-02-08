# grpc_mem_bench


## Building

- Rust
```sh
docker build -f dockerfiles/rust.Dockerfile --build-arg PROJECT_DIR=rust_tonic --build-arg PROJECT_NAME=grpc_mem_bench_rust_tonic -t grpc_mem_bench_rust_tonic:latest .
```