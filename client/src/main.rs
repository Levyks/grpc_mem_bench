use std::env;
use clap::Parser;
use grpc_mem_bench_client::args::Args;
use grpc_mem_bench_client::bench::bench;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();
    
    bench(Args::parse()).await
}
