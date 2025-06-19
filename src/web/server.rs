use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::core::models::DeviceInfo;
use crate::web::routes::create_routes;

pub struct WebServer {
    addr: SocketAddr,
    directory: PathBuf,
    device_info: DeviceInfo,
    max_file_size: u64,
}

impl WebServer {
    pub fn new(addr: SocketAddr, directory: PathBuf, device_info: DeviceInfo, max_file_size: u64) -> Self {
        Self {
            addr,
            directory,
            device_info,
            max_file_size,
        }
    }
    
    pub async fn run(&self) -> Result<()> {
        // Create CORS layer
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        
        // Create the application router
        let app = create_routes(self.directory.clone(), self.device_info.clone(), self.max_file_size)
            .layer(TraceLayer::new_for_http())
            .layer(cors);
        
        // Start the server
        info!("Starting web server on {}", self.addr);
        let listener = TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}
