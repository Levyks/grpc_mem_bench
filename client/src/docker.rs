use docker_api::{Container, Docker, Result, Error};
use docker_api::opts::PublishPort;

pub async fn create_container(docker: &Docker, image: &str, memory: u64, expose: Option<(u32, u32)>) -> Result<Container> {
    use docker_api::opts::ContainerCreateOpts;
    
    let mut builder = ContainerCreateOpts::builder()
        .image(image)
        .auto_remove(true)
        .memory(memory);
    
    if let Some((container_port, host_port)) = expose {
        builder = builder.expose(PublishPort::tcp(container_port), host_port);
    }
    
    let opts = builder.build();

    let container = docker.containers().create(&opts).await?;
    
    container.start().await?;
    
    Ok(container)
}

pub async fn stop_container(container: &Container) -> Result<()> {
    use docker_api::opts::ContainerStopOpts;
    let opts = ContainerStopOpts::builder().build();
    container.stop(&opts).await
}

pub async fn get_container_bridge_ip(container: &Container) -> Result<String> {
    let details = container.inspect().await?;

    let ip = details.network_settings.as_ref()
        .and_then(|networks| networks.networks.as_ref())
        .and_then(|networks| networks.get("bridge"))
        .and_then(|network| network.ip_address.clone()) // C
        .ok_or_else(|| Error::StringError("Container has no bridge network or IP address".into()))?;

    Ok(ip)
}

pub fn parse_memory_limit(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    let (num, unit) = s.split_at(s.chars().position(|c| !c.is_numeric()).unwrap_or(s.len()));

    let num: u64 = num.parse().ok()?;
    let multiplier = match unit {
        "K" => 1024,
        "M" => 1024 * 1024,
        "G" => 1024 * 1024 * 1024,
        "" => 1, // If no unit, assume bytes
        _ => return None, // Unsupported unit
    };

    Some(num * multiplier)
}