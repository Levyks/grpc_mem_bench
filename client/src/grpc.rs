use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::time::{Duration, timeout, interval};
use tonic::Request;
use tonic::transport::Channel;
use crate::bench::Bench;
use crate::proto::grpc_mem_bench_service_client::GrpcMemBenchServiceClient;
use crate::proto::PingWithInterval;

pub async fn wait_for_server(address: &str, interval_duration: Duration, timeout_duration: Duration) -> anyhow::Result<()> {
    timeout(timeout_duration, async {
        let mut interval = interval(interval_duration);
        loop {
            match connect_and_health_check(address).await {
                Ok(_) => {
                    break;
                },
                Err(e) => {
                    interval.tick().await;
                }
            }
        }
    }).await?;
    
    Ok(())
}


pub async fn connect(address: &str) -> anyhow::Result<GrpcMemBenchServiceClient<Channel>> {
    let client = GrpcMemBenchServiceClient::connect(address.to_string()).await?;
    Ok(client)
}

async fn connect_and_health_check(address: &str) -> anyhow::Result<()> {
    connect(address).await?.health_check(Request::new(())).await?;
    Ok(())
}

pub async fn connect_and_run_server_streaming(address: &str, message: &str, interval: Duration, bench: Arc<Bench>) -> anyhow::Result<()> {
    let mut client = connect(address).await?;
    let mut stream = client.server_stream(
        Request::new(PingWithInterval {
            message: message.to_string(),
            interval: 1000,
        })
    ).await?.into_inner();

    let cloned_message = message.to_string();
    tokio::spawn(async move {
        loop {
            match stream.message().await {
                Ok(Some(_)) => {
                    bench.add_pong();
                },
                _ => {
                    bench.fail(&cloned_message).await;
                },
            }
        }
    });

    Ok(())
}