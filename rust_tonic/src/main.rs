
mod proto {
    tonic::include_proto!("grpc_mem_bench");
}

use std::env;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::time::{self, Duration, Interval};
use tokio_stream::{Stream, StreamExt};
use tokio_stream::adapters::Map;
use tonic::{Request, Response, Status, Streaming};
use tonic::transport::Server;
use proto::grpc_mem_bench_service_server::GrpcMemBenchService;
use crate::proto::{Ping, PingWithInterval, Pong};
use crate::proto::grpc_mem_bench_service_server::GrpcMemBenchServiceServer;

struct GrpcMemBenchServiceImpl;

#[tonic::async_trait]
impl GrpcMemBenchService for GrpcMemBenchServiceImpl {
    type ServerStreamStream = PingIntervalStream;

    async fn server_stream(&self, request: Request<PingWithInterval>) -> Result<Response<Self::ServerStreamStream>, Status> {
        let ping = request.into_inner();

        Ok(Response::new(PingIntervalStream::new(ping)))
    }

    type BiDirectionalStreamStream = Map<Streaming<Ping>, fn(Result<Ping, Status>) -> Result<Pong, Status>>;

    async fn bi_directional_stream(&self, request: Request<Streaming<Ping>>) -> Result<Response<Self::BiDirectionalStreamStream>, Status> {
        let stream = request.into_inner();

        let response_stream: Self::BiDirectionalStreamStream = stream.map( |result| {
            result
                .map(|ping| Pong {
                    message: ping.message,
                    timestamp: Some(prost_types::Timestamp::from(SystemTime::now()))
                })
                .map_err(|e| Status::internal(format!("Stream error: {}", e)))
        });

        Ok(Response::new(response_stream))
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .map(|s| s.parse().expect("Invalid PORT env variable"))
        .unwrap_or(50051);

    let listen_address: SocketAddr = format!("[::]:{}", port).parse().unwrap();

    println!("Server listening on {}", listen_address);

    Server::builder()
        .add_service(GrpcMemBenchServiceServer::new(GrpcMemBenchServiceImpl))
        .serve(listen_address)
        .await
        .unwrap();
}

pub struct PingIntervalStream {
    message: String,
    inner: Interval,
}

impl PingIntervalStream {
    pub fn new(ping_with_interval: PingWithInterval) -> Self {
        let duration = Duration::from_millis(ping_with_interval.interval as u64);
        Self {
            message: ping_with_interval.message,
            inner: time::interval(duration),
        }
    }
}

impl Stream for PingIntervalStream {
    type Item = Result<Pong, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Pong, Status>>> {
        self.inner.poll_tick(cx).map(|_| Some(Ok(Pong {
            message: self.message.clone(),
            timestamp: Some(prost_types::Timestamp::from(SystemTime::now())),
        })))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}