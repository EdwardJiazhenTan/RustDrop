use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tracing::{info, error, warn};

use crate::core::models::DeviceInfo;
use crate::discovery::ServiceDiscovery;
use crate::utils::qrcode::generate_qr_code;
use crate::web::server::WebServer;

pub struct App {
    port: u16,
    directory: PathBuf,
    enable_mdns: bool,
    enable_qr: bool,
    open_browser: bool,
    max_file_size: u64,
    device_info: DeviceInfo,
}

impl App {
    pub fn new(
        port: u16,
        directory: PathBuf,
        enable_mdns: bool,
        enable_qr: bool,
        open_browser: bool,
        max_file_size: u64,
    ) -> Self {
        let device_info = DeviceInfo::new(port);
        
        Self {
            port,
            directory,
            enable_mdns,
            enable_qr,
            open_browser,
            max_file_size,
            device_info,
        }
    }
    
    pub async fn run(&self) -> Result<()> {
        // Print application information
        info!("Serving files from: {:?}", self.directory);
        info!("Web interface available at: {}", self.device_info.url());
        
        // Display QR code if enabled
        if self.enable_qr {
            match generate_qr_code(&self.device_info.url()) {
                Ok(qr_code) => println!("{}", qr_code),
                Err(e) => error!("Failed to generate QR code: {}", e),
            }
        }
        
        // Start mDNS service discovery if enabled
        let mut discovery = if self.enable_mdns {
            let mut service = ServiceDiscovery::new(self.device_info.clone());
            match service.register().await {
                Ok(_) => {
                    info!("mDNS service registered successfully");
                    Some(service)
                },
                Err(e) => {
                    error!("Failed to register mDNS service: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // Open browser if requested
        if self.open_browser {
            if let Err(e) = open::that(&self.device_info.url()) {
                error!("Failed to open browser: {}", e);
            }
        }
        
        // Start the web server
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let server = WebServer::new(addr, self.directory.clone(), self.device_info.clone(), self.max_file_size);
        
        // Setup graceful shutdown
        let shutdown_signal = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            info!("Received Ctrl+C, shutting down gracefully...");
        };
        
        // Run the server with graceful shutdown
        tokio::select! {
            result = server.run() => {
                if let Err(e) = result {
                    error!("Server error: {}", e);
                }
            }
            _ = shutdown_signal => {
                info!("Shutdown signal received");
            }
        }
        
        // Graceful cleanup
        info!("Cleaning up services...");
        
        // Unregister mDNS service if it was started
        if let Some(ref mut discovery) = discovery {
            info!("Unregistering mDNS service...");
            if let Err(e) = discovery.unregister().await {
                warn!("Failed to unregister mDNS service: {}", e);
            } else {
                info!("mDNS service unregistered successfully");
            }
            
            // Give a moment for mDNS cleanup
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        info!("Shutdown complete");
        Ok(())
    }
}
