pub mod args;
pub mod bench;
mod docker;
mod grpc;
mod utils;

mod proto {
    tonic::include_proto!("grpc_mem_bench");
}
