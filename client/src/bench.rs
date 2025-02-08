use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Duration;
use tokio::sync::broadcast::{Receiver, Sender};
use crate::args::Args;
use crate::docker::{create_container, get_container_bridge_ip, parse_memory_limit, stop_container};
use crate::grpc::{wait_for_server, connect, connect_and_run_server_streaming};
use crate::utils::generate_random_port;

pub async fn bench(args: Args) -> anyhow::Result<()> {
    let docker = docker_api::Docker::new(args.docker)?;

    let (expose, server_port) = if args.expose_port {
        let random_port = generate_random_port().await;
        (Some((args.port, random_port)), random_port)
    } else {
        (None, args.port)
    };

    let memory_limit = parse_memory_limit(&args.memory).ok_or_else(|| anyhow::anyhow!("Invalid memory limit"))?;

    log::info!("Creating container with image: {}, memory limit: {}, expose: {:?}", args.image, memory_limit, expose);
    let container = create_container(&docker, &args.image, memory_limit, expose).await?;
    log::info!("Container created with id: {}", container.id());

    let ip = if args.expose_port {
        "127.0.0.1".into()
    } else {
        get_container_bridge_ip(&container).await?
    };

    let address = format!("http://{}:{}", ip, server_port);
    log::info!("Waiting for server to be ready at {}", address);
    wait_for_server(&address, args.startup_check_interval, args.startup_check_interval).await?;
    log::info!("Server is ready");

    let bench = Arc::new(Bench::new(container, address));
    
    let parallelism = 8;
    for _ in 0..parallelism {
        let bench = bench.clone();
        tokio::spawn(async move {
            Bench::connection_loop(bench, Duration::from_millis(5)).await
        });
    }
    
    let cloned_bench = bench.clone();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to capture ctrl-c");
        cloned_bench.fail("ctrl-c").await;
    });
    
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        bench.report_and_reset();
    }
}

pub struct Bench {
    container: docker_api::Container,
    address: String,
    number_of_streams: AtomicU64,
    connections_since_last_report: AtomicU64,
    connection_time_since_last_report: AtomicU64,
    pongs_received_since_last_report: AtomicU64,
    is_shutting_down: AtomicBool,
}

impl Bench {
    fn new(container: docker_api::Container, address: String) -> Self {
        Self {
            container,
            address,
            number_of_streams: AtomicU64::new(0),
            connections_since_last_report: AtomicU64::new(0),
            connection_time_since_last_report: AtomicU64::new(0),
            pongs_received_since_last_report: AtomicU64::new(0),
            is_shutting_down: AtomicBool::new(false),
        }
    }

    fn report_and_reset(&self) {
        let number_of_streams = self.number_of_streams.load(std::sync::atomic::Ordering::Relaxed);
        let pongs_received_since_last_report = self.pongs_received_since_last_report.swap(0, std::sync::atomic::Ordering::Relaxed);
        let connections_since_last_report = self.connections_since_last_report.swap(0, std::sync::atomic::Ordering::Relaxed);
        let connection_time_since_last_report = self.connection_time_since_last_report.swap(0, std::sync::atomic::Ordering::Relaxed);

        let avg_connection_time = if connections_since_last_report > 0 {
            connection_time_since_last_report / connections_since_last_report
        } else {
            0
        };

        log::info!(
            "Number of streams: {}, pongs received since last report: {}, average connection time: {} ({} connections since last report)", 
            number_of_streams, pongs_received_since_last_report, avg_connection_time, connections_since_last_report
        );
    }

    async fn connection_loop(arc_self: Arc<Self>, interval_duration: Duration) {
        let mut interval = tokio::time::interval(interval_duration);
        let mut idx = 0;
        loop {
            // measure time to connect 
            let start = std::time::Instant::now();
            if let Err(err) = connect_and_run_server_streaming(
                &arc_self.address, 
                format!("stream-{}", idx).as_str(),
                Duration::from_secs(1), 
                arc_self.clone()
            ).await {
                log::warn!("Connection failed in idx {}: {:?}", idx, err);
                arc_self.fail(format!("stream-{}", idx).as_str()).await;
            }
            let elapsed = start.elapsed().as_millis() as u64;
            arc_self.connection_time_since_last_report.fetch_add(elapsed, std::sync::atomic::Ordering::Relaxed);
            arc_self.connections_since_last_report.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            arc_self.number_of_streams.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            idx += 1;
            interval.tick().await;
        }
    }

    pub(crate) fn add_pong(&self) {
        self.pongs_received_since_last_report.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) async fn fail(&self, stream_id: &str) {
        log::warn!("Stream {} failed", stream_id);
        let is_already_shutting_down = self.is_shutting_down.swap(true, std::sync::atomic::Ordering::Relaxed);
        
        if !is_already_shutting_down {
            self.report_and_reset();
            stop_container(&self.container).await.expect("Failed to stop container");
            std::process::exit(1);
        }
    }

}