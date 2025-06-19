//! RustDrop - Cross-platform file transfer tool with web interface
//! 
//! This crate provides a simple and secure way to share files across devices
//! on a local network using a web interface.

pub mod core;
pub mod utils;
pub mod web;
pub mod discovery;
pub mod cli;

// Re-export commonly used types for convenience
pub use core::{
    config::AppConfig,
    models::{DeviceInfo, FileInfo},
    error::{AppError, AppResult},
};

pub use utils::{
    file::{get_file_info, list_directory},
    network::{find_available_port, is_port_available},
};

pub use web::{
    routes::create_routes,
    server::WebServer,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "rustdrop");
        assert!(!DESCRIPTION.is_empty());
    }

    #[test]
    fn test_module_availability() {
        // Test that we can create basic types
        let _config = AppConfig::default();
        let _device = DeviceInfo::new(8080);
        
        // Test utility functions are available
        assert!(is_port_available(65432)); // High port should be available
    }
} 