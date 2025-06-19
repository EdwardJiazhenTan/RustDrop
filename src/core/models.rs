use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub size_human: String,
    pub modified: DateTime<Utc>,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub os: String,
}

impl DeviceInfo {
    pub fn new(port: u16) -> Self {
        let hostname = hostname::get()
            .unwrap_or_else(|_| "unknown".into())
            .to_string_lossy()
            .to_string();
            
        let ip = local_ip_address::local_ip()
            .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
            .to_string();
            
        let os = std::env::consts::OS.to_string();
        
        Self {
            id: Uuid::new_v4().to_string(),
            name: hostname,
            ip,
            port,
            os,
        }
    }
    
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.ip, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    #[test]
    fn test_file_info_creation() {
        let file_info = FileInfo {
            id: Uuid::new_v4(),
            name: "test.txt".to_string(),
            path: PathBuf::from("/tmp/test.txt"),
            size: 1024,
            size_human: "1.0 KiB".to_string(),
            modified: Utc::now(),
            mime_type: "text/plain".to_string(),
        };

        assert_eq!(file_info.name, "test.txt");
        assert_eq!(file_info.path, PathBuf::from("/tmp/test.txt"));
        assert_eq!(file_info.size, 1024);
        assert_eq!(file_info.size_human, "1.0 KiB");
        assert_eq!(file_info.mime_type, "text/plain");
    }

    #[test]
    fn test_file_info_serialization() {
        let file_info = FileInfo {
            id: Uuid::new_v4(),
            name: "example.json".to_string(),
            path: PathBuf::from("/home/user/example.json"),
            size: 2048,
            size_human: "2.0 KiB".to_string(),
            modified: Utc::now(),
            mime_type: "application/json".to_string(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&file_info).unwrap();
        assert!(json.contains("example.json"));
        assert!(json.contains("application/json"));
        assert!(json.contains("2048"));

        // Test deserialization
        let deserialized: FileInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, file_info.name);
        assert_eq!(deserialized.mime_type, file_info.mime_type);
        assert_eq!(deserialized.size, file_info.size);
    }

    #[test]
    fn test_device_info_creation() {
        let port = 8080;
        let device_info = DeviceInfo::new(port);

        assert_eq!(device_info.port, port);
        assert!(!device_info.id.is_empty());
        assert!(!device_info.name.is_empty());
        assert!(!device_info.ip.is_empty());
        assert!(!device_info.os.is_empty());

        // Verify UUID format
        assert!(Uuid::parse_str(&device_info.id).is_ok());
        
        // Verify OS is one of the expected values
        let valid_os = ["linux", "macos", "windows", "freebsd", "openbsd", "netbsd"];
        assert!(valid_os.contains(&device_info.os.as_str()));
    }

    #[test]
    fn test_device_info_url_generation() {
        let port = 9090;
        let device_info = DeviceInfo::new(port);
        let url = device_info.url();

        assert!(url.starts_with("http://"));
        assert!(url.contains(&port.to_string()));
        assert!(url.contains(&device_info.ip));

        // Verify URL format
        let expected_url = format!("http://{}:{}", device_info.ip, device_info.port);
        assert_eq!(url, expected_url);
    }

    #[test]
    fn test_device_info_serialization() {
        let device_info = DeviceInfo::new(3000);

        // Test JSON serialization
        let json = serde_json::to_string(&device_info).unwrap();
        assert!(json.contains("3000"));
        assert!(json.contains(&device_info.name));
        assert!(json.contains(&device_info.ip));
        assert!(json.contains(&device_info.os));

        // Test deserialization
        let deserialized: DeviceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, device_info.id);
        assert_eq!(deserialized.name, device_info.name);
        assert_eq!(deserialized.ip, device_info.ip);
        assert_eq!(deserialized.port, device_info.port);
        assert_eq!(deserialized.os, device_info.os);
    }

    #[test]
    fn test_device_info_clone() {
        let original = DeviceInfo::new(4000);
        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.ip, cloned.ip);
        assert_eq!(original.port, cloned.port);
        assert_eq!(original.os, cloned.os);
    }

    #[test]
    fn test_file_info_clone() {
        let original = FileInfo {
            id: Uuid::new_v4(),
            name: "clone_test.txt".to_string(),
            path: PathBuf::from("/test/clone_test.txt"),
            size: 512,
            size_human: "512 B".to_string(),
            modified: Utc::now(),
            mime_type: "text/plain".to_string(),
        };

        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.size, cloned.size);
        assert_eq!(original.size_human, cloned.size_human);
        assert_eq!(original.modified, cloned.modified);
        assert_eq!(original.mime_type, cloned.mime_type);
    }

    #[test]
    fn test_different_device_info_have_different_ids() {
        let device1 = DeviceInfo::new(8080);
        let device2 = DeviceInfo::new(8081);

        // Different devices should have different IDs
        assert_ne!(device1.id, device2.id);
        
        // But same IP and OS (assuming same machine)
        assert_eq!(device1.ip, device2.ip);
        assert_eq!(device1.os, device2.os);
    }

    #[test]
    fn test_device_info_with_different_ports() {
        let ports = [80, 443, 8080, 9000, 65535];
        
        for port in ports {
            let device_info = DeviceInfo::new(port);
            assert_eq!(device_info.port, port);
            
            let url = device_info.url();
            assert!(url.ends_with(&format!(":{}", port)));
        }
    }

    #[test]
    fn test_file_info_path_handling() {
        let test_cases = vec![
            ("/home/user/document.pdf", "document.pdf"),
            ("C:\\Users\\User\\file.txt", "file.txt"),
            ("./relative/path/image.png", "image.png"),
            ("/single_file.json", "single_file.json"),
        ];

        for (path_str, expected_name) in test_cases {
            let path = PathBuf::from(path_str);
            let file_info = FileInfo {
                id: Uuid::new_v4(),
                name: expected_name.to_string(),
                path: path.clone(),
                size: 100,
                size_human: "100 B".to_string(),
                modified: Utc::now(),
                mime_type: "application/octet-stream".to_string(),
            };

            assert_eq!(file_info.name, expected_name);
            assert_eq!(file_info.path, path);
        }
    }
}
