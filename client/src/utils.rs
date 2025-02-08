use rand::Rng;
use tokio::net::TcpListener;

pub async fn generate_random_port() -> u32 {
    loop {
        let port = rand::rng().random_range(49152..=65535);
        let addr = format!("127.0.0.1:{}", port);

        if TcpListener::bind(&addr).await.is_ok() {
            return port;
        }
    }
}