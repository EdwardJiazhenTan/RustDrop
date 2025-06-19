mod cli;
mod core;
mod discovery;
mod utils;
mod web;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with custom filter to reduce mDNS noise during shutdown
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Filter out harmless mDNS errors during shutdown
            EnvFilter::new("info,mdns_sd::service_daemon=off")
        });
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .init();

    let cli = Cli::parse();
    cli.run().await
}
