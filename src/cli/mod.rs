use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use crate::core::app::App;
use crate::core::config::AppConfig;
use crate::utils::network::get_available_port_or_default;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Port to listen on (will find next available port if this one is in use)
    #[arg(short, long)]
    port: Option<u16>,

    /// Directory to serve files from
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// Disable mDNS service discovery
    #[arg(long)]
    no_mdns: bool,

    /// Disable QR code display
    #[arg(long)]
    no_qr: bool,

    /// Open web browser automatically
    #[arg(short, long)]
    open: bool,

    /// Generate example configuration file
    #[arg(long)]
    generate_config: bool,
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        // Generate config file if requested
        if self.generate_config {
            AppConfig::save_example()?;
            println!("Generated example configuration file: rustdrop.example.toml");
            return Ok(());
        }

        // Load configuration
        let mut config = AppConfig::load().unwrap_or_else(|e| {
            info!("Using default configuration ({})", e);
            AppConfig::default()
        });

        // Override config with CLI arguments
        if let Some(port) = self.port {
            config.server.port = port;
        }
        if let Some(ref directory) = self.directory {
            config.files.directory = Some(directory.clone());
        }
        if self.no_mdns {
            config.discovery.enabled = false;
        }
        if self.no_qr {
            config.ui.qr_code = false;
        }
        if self.open {
            config.ui.open_browser = true;
        }

        // Determine the directory to serve files from
        let directory = config.files.directory.clone().unwrap_or_else(|| {
            let current_dir = std::env::current_dir().expect("Failed to get current directory");
            info!("No directory specified, using current directory: {:?}", current_dir);
            current_dir
        });

        // Find an available port
        let available_port = get_available_port_or_default(config.server.port);
        
        // Create and run the application
        let app = App::new(
            available_port,
            directory,
            config.discovery.enabled,
            config.ui.qr_code,
            config.ui.open_browser,
            config.server.max_file_size,
        );
        
        app.run().await
    }
}
