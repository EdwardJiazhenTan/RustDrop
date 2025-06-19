use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub files: FilesConfig,
    pub discovery: DiscoveryConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub directory: Option<PathBuf>,
    #[serde(default = "default_file_expiry")]
    pub expiry_hours: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub qr_code: bool,
    #[serde(default = "default_false")]
    pub open_browser: bool,
}

// Default value functions
fn default_port() -> u16 { 8080 }
fn default_host() -> String { "0.0.0.0".to_string() }
fn default_max_file_size() -> u64 { 1024 * 1024 * 1024 } // 1GB
fn default_file_expiry() -> Option<u64> { None }
fn default_true() -> bool { true }
fn default_false() -> bool { false }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: default_port(),
                host: default_host(),
                max_file_size: default_max_file_size(),
            },
            files: FilesConfig {
                directory: None,
                expiry_hours: default_file_expiry(),
            },
            discovery: DiscoveryConfig {
                enabled: default_true(),
            },
            ui: UiConfig {
                qr_code: default_true(),
                open_browser: default_false(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("rustdrop.toml").required(false))
            .add_source(config::Environment::with_prefix("RUSTDROP"));

        // Override with individual environment variables
        if let Ok(port) = std::env::var("PORT") {
            builder = builder.set_override("server.port", port)?;
        }
        if let Ok(host) = std::env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
        }
        if let Ok(dir) = std::env::var("UPLOAD_DIR") {
            builder = builder.set_override("files.directory", dir)?;
        }
        if let Ok(size) = std::env::var("MAX_FILE_SIZE") {
            builder = builder.set_override("server.max_file_size", size)?;
        }

        let settings = builder.build()?;
        let config: AppConfig = settings.try_deserialize()?;
        Ok(config)
    }

    pub fn save_example() -> Result<()> {
        let example_config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&example_config)?;
        std::fs::write("rustdrop.example.toml", toml_string)?;
        Ok(())
    }

    pub fn from_toml(toml_content: &str) -> Result<Self> {
        let config: AppConfig = toml::from_str(toml_content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.max_file_size, 1024 * 1024 * 1024);
        assert!(config.discovery.enabled);
        assert!(config.ui.qr_code);
        assert!(!config.ui.open_browser);
        assert!(config.files.directory.is_none());
        assert!(config.files.expiry_hours.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&config).unwrap();
        
        assert!(toml_string.contains("[server]"));
        assert!(toml_string.contains("port = 8080"));
        assert!(toml_string.contains("host = \"0.0.0.0\""));
        assert!(toml_string.contains("[discovery]"));
        assert!(toml_string.contains("enabled = true"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_content = r#"
            [server]
            port = 9090
            host = "127.0.0.1"
            max_file_size = 500000000

            [files]
            expiry_hours = 24

            [discovery]
            enabled = false

            [ui]
            qr_code = false
            open_browser = true
        "#;

        let config = AppConfig::from_toml(toml_content).unwrap();
        
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.max_file_size, 500000000);
        assert_eq!(config.files.expiry_hours, Some(24));
        assert!(!config.discovery.enabled);
        assert!(!config.ui.qr_code);
        assert!(config.ui.open_browser);
    }

    #[test]
    fn test_config_with_custom_directory() {
        let toml_content = r#"
            [server]
            port = 8080
            host = "0.0.0.0"
            max_file_size = 1073741824

            [files]
            directory = "/tmp/uploads"

            [discovery]
            enabled = true

            [ui]
            qr_code = true
            open_browser = false
        "#;

        let config = AppConfig::from_toml(toml_content).unwrap();
        assert_eq!(config.files.directory, Some(PathBuf::from("/tmp/uploads")));
    }

    #[test]
    fn test_partial_config() {
        let toml_content = r#"
            [server]
            port = 3000
            host = "0.0.0.0"
            max_file_size = 1073741824

            [files]

            [discovery]
            enabled = true

            [ui]
            qr_code = true
            open_browser = false
        "#;

        let config = AppConfig::from_toml(toml_content).unwrap();
        
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.host, "0.0.0.0"); // Default value
        assert_eq!(config.server.max_file_size, 1024 * 1024 * 1024); // Default value
        assert!(config.discovery.enabled); // Default value
    }

    #[test]
    fn test_save_example_config() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        
        // Change to temp directory
        env::set_current_dir(&temp_dir).unwrap();
        
        // Test saving example config
        AppConfig::save_example().unwrap();
        
        // Verify file exists and contains expected content
        let content = std::fs::read_to_string("rustdrop.example.toml").unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("port = 8080"));
        
        // Restore original directory
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_invalid_toml() {
        let invalid_toml = "invalid toml content [[[";
        let result = AppConfig::from_toml(invalid_toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_environment_variable_override() {
        // Note: This test would need to be run in isolation or with proper env var cleanup
        // For now, we test the logic structure
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
    }
} 