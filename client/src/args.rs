use clap::Parser;
use humantime::parse_duration;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub(crate) image: String,

    #[arg(short, long)]
    pub(crate) memory: String,

    #[arg(short, long, default_value_t = 50051)]
    pub(crate) port: u32,

    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub(crate) expose_port: bool,

    #[arg(short, long, default_value = "unix:///var/run/docker.sock")]
    pub(crate) docker: String,

    #[arg(long, value_parser = parse_duration, default_value = "1s")]
    pub(crate) startup_check_interval: Duration,
    
    #[arg(long, value_parser = parse_duration, default_value = "10s")]
    pub(crate) startup_check_timeout: Duration,
}