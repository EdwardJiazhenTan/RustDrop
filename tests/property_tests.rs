use proptest::prelude::*;
use rustdrop::core::config::AppConfig;
use rustdrop::core::models::DeviceInfo;
use rustdrop::utils::file::get_file_info;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use uuid::Uuid;
use rustdrop::core::config::{ServerConfig, FilesConfig, DiscoveryConfig, UiConfig};

// Property test for file info consistency
proptest! {
    #[test]
    fn test_file_info_deterministic_uuid(
        filename in "[a-zA-Z0-9_.-]{1,50}\\.txt",
        content in ".*{0,1000}"
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(&filename);
        
        // Create file with content
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        drop(file);
        
        // Get file info multiple times
        let info1 = get_file_info(&file_path).unwrap();
        let info2 = get_file_info(&file_path).unwrap();
        let info3 = get_file_info(&file_path).unwrap();
        
        // UUIDs should be identical for the same file
        prop_assert_eq!(info1.id, info2.id);
        prop_assert_eq!(info2.id, info3.id);
        
        // File properties should match
        prop_assert_eq!(info1.name, filename);
        prop_assert_eq!(info1.size, content.len() as u64);
        prop_assert_eq!(info1.mime_type, "text/plain");
    }
}

proptest! {
    #[test]
    fn test_file_size_accuracy(
        size in 0u64..10_000_000
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("size_test.bin");
        
        // Create file of exact size
        let data = vec![0u8; size as usize];
        std::fs::write(&file_path, &data).unwrap();
        
        let info = get_file_info(&file_path).unwrap();
        
        // Size should match exactly
        prop_assert_eq!(info.size, size);
        
        // Size human should end with appropriate unit
        prop_assert!(info.size_human.ends_with("B"));
        
        // Check size human formatting makes sense
        if size == 0 {
            prop_assert_eq!(info.size_human, "0 B");
        } else if size < 1024 {
            prop_assert!(info.size_human.contains(&size.to_string()));
        }
    }
}

// Property test for device info generation
proptest! {
    #[test]
    fn test_device_info_properties(
        port in 1u16..65535
    ) {
        let device = DeviceInfo::new(port);
        
        // Port should match input
        prop_assert_eq!(device.port, port);
        
        // ID should be valid UUID
        prop_assert!(Uuid::parse_str(&device.id).is_ok());
        
        // Name should not be empty
        prop_assert!(!device.name.is_empty());
        
        // IP should not be empty
        prop_assert!(!device.ip.is_empty());
        
        // OS should not be empty
        prop_assert!(!device.os.is_empty());
        
        // URL should be valid format
        let url = device.url();
        prop_assert!(url.starts_with("http://"));
        prop_assert!(url.contains(&port.to_string()));
        prop_assert!(url.contains(&device.ip));
    }
}

proptest! {
    #[test]
    fn test_device_info_uniqueness(
        ports in prop::collection::vec(1u16..65535, 1..100)
    ) {
        let devices: Vec<DeviceInfo> = ports.iter().map(|&port| DeviceInfo::new(port)).collect();
        
        // All device IDs should be unique
        let mut ids = std::collections::HashSet::new();
        for device in &devices {
            prop_assert!(ids.insert(device.id.clone()));
        }
        
        // Ports should match input
        for (device, &expected_port) in devices.iter().zip(ports.iter()) {
            prop_assert_eq!(device.port, expected_port);
        }
    }
}

// Property test for configuration
proptest! {
    #[test]
    fn test_config_port_ranges(
        port in 1u16..65535,
        max_file_size in 1u64..10_000_000_000,
        expiry_hours in prop::option::of(1u64..8760) // Up to 1 year
    ) {
        let toml_content = format!(r#"
            [server]
            port = {}
            host = "0.0.0.0"
            max_file_size = {}

            [files]
            directory = ""
            {}

            [discovery]
            enabled = true

            [ui]
            qr_code = true
            open_browser = false
        "#, 
            port, 
            max_file_size,
            expiry_hours.map_or(String::new(), |h| format!("expiry_hours = {}", h))
        );
        
        let config = AppConfig::from_toml(&toml_content).unwrap();
        
        // Values should match input
        prop_assert_eq!(config.server.port, port);
        prop_assert_eq!(config.server.max_file_size, max_file_size);
        prop_assert_eq!(config.files.expiry_hours, expiry_hours);
    }
}

proptest! {
    #[test]
    fn test_config_serialization_roundtrip(
        port in 1u16..65535,
        max_file_size in 1u64..10_000_000_000,
        enabled in any::<bool>(),
        qr_code in any::<bool>(),
        open_browser in any::<bool>()
    ) {
        let original_config = AppConfig {
            server: ServerConfig {
                port,
                host: "127.0.0.1".to_string(),
                max_file_size,
            },
            files: FilesConfig {
                directory: None,
                expiry_hours: Some(24),
            },
            discovery: DiscoveryConfig {
                enabled,
            },
            ui: UiConfig {
                qr_code,
                open_browser,
            },
        };
        
        // Serialize to TOML and back
        let toml_string = toml::to_string(&original_config).unwrap();
        let parsed_config: AppConfig = toml::from_str(&toml_string).unwrap();
        
        // Should be identical
        prop_assert_eq!(original_config.server.port, parsed_config.server.port);
        prop_assert_eq!(original_config.server.host, parsed_config.server.host);
        prop_assert_eq!(original_config.server.max_file_size, parsed_config.server.max_file_size);
        prop_assert_eq!(original_config.discovery.enabled, parsed_config.discovery.enabled);
        prop_assert_eq!(original_config.ui.qr_code, parsed_config.ui.qr_code);
        prop_assert_eq!(original_config.ui.open_browser, parsed_config.ui.open_browser);
    }
}

// Property test for file path handling
proptest! {
    #[test]
    fn test_file_path_edge_cases(
        filename in r"[a-zA-Z0-9_\-\.]{1,100}",
        extension in r"[a-zA-Z0-9]{1,10}"
    ) {
        let temp_dir = TempDir::new().unwrap();
        let full_filename = format!("{}.{}", filename, extension);
        let file_path = temp_dir.path().join(&full_filename);
        
        // Create empty file
        File::create(&file_path).unwrap();
        
        let info = get_file_info(&file_path).unwrap();
        
        // Name should match full filename
        prop_assert_eq!(info.name, full_filename);
        
        // Path should be absolute and exist
        prop_assert!(info.path.is_absolute());
        prop_assert!(info.path.exists());
        
        // Size should be 0 for empty file
        prop_assert_eq!(info.size, 0);
        prop_assert_eq!(info.size_human, "0 B");
    }
}

// Property test for concurrent file operations
proptest! {
    #[test]
    fn test_concurrent_file_info_consistency(
        filenames in prop::collection::vec(r"[a-zA-Z0-9_]{1,20}\.txt", 1..20)
    ) {
        let temp_dir = TempDir::new().unwrap();
        
        // Create files concurrently
        use std::sync::Arc;
        use std::thread;
        
        let temp_path = Arc::new(temp_dir.path().to_path_buf());
        let filenames = Arc::new(filenames);
        
        // Create files with unique names by adding index
        for (index, filename) in filenames.iter().enumerate() {
            let unique_filename = format!("{}_{}", index, filename);
            let file_path = temp_dir.path().join(&unique_filename);
            std::fs::write(&file_path, format!("Content for {}", unique_filename)).unwrap();
        }
        
        // Get file info concurrently for the unique filenames
        let handles: Vec<_> = filenames.iter().enumerate().map(|(index, filename)| {
            let path = temp_path.clone();
            let unique_name = format!("{}_{}", index, filename);
            thread::spawn(move || {
                let file_path = path.join(&unique_name);
                get_file_info(&file_path)
            })
        }).collect();
        
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        
        // All operations should succeed
        for result in &results {
            prop_assert!(result.is_ok());
        }
        
        let infos: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
        
        // All UUIDs should be unique (different files with unique names)
        let mut ids = std::collections::HashSet::new();
        for info in &infos {
            prop_assert!(ids.insert(info.id.clone()), "UUID should be unique for file: {}", info.name);
        }
        
        // Names should match the unique filenames we created
        let mut names: Vec<_> = infos.iter().map(|i| i.name.clone()).collect();
        let mut expected_names: Vec<_> = filenames.iter().enumerate()
            .map(|(index, filename)| format!("{}_{}", index, filename))
            .collect();
        names.sort();
        expected_names.sort();
        prop_assert_eq!(names, expected_names);
    }
}

// Property test for MIME type detection
proptest! {
    #[test]
    fn test_mime_type_detection(
        basename in r"[a-zA-Z0-9_\-]{1,50}",
        extension in r"(txt|json|html|css|js|png|jpg|pdf|zip|bin)"
    ) {
        let temp_dir = TempDir::new().unwrap();
        let filename = format!("{}.{}", basename, extension);
        let file_path = temp_dir.path().join(&filename);
        
        // Create empty file
        File::create(&file_path).unwrap();
        
        let info = get_file_info(&file_path).unwrap();
        
        // MIME type should be appropriate for extension
        match extension.as_str() {
            "txt" => prop_assert_eq!(info.mime_type, "text/plain"),
            "json" => prop_assert_eq!(info.mime_type, "application/json"),
            "html" => prop_assert_eq!(info.mime_type, "text/html"),
            "css" => prop_assert_eq!(info.mime_type, "text/css"),
            "js" => prop_assert_eq!(info.mime_type, "text/javascript"),
            "png" => prop_assert_eq!(info.mime_type, "image/png"),
            "jpg" => prop_assert_eq!(info.mime_type, "image/jpeg"),
            "pdf" => prop_assert_eq!(info.mime_type, "application/pdf"),
            "zip" => prop_assert_eq!(info.mime_type, "application/zip"),
            "bin" => prop_assert_eq!(info.mime_type, "application/octet-stream"),
            _ => prop_assert!(!info.mime_type.is_empty()),
        }
    }
}

// Property test for UUID determinism across different file content
proptest! {
    #[test]
    fn test_uuid_path_determinism(
        path_parts in prop::collection::vec(r"[a-zA-Z0-9_\-]{1,20}", 1..5),
        filename in r"[a-zA-Z0-9_\-]{1,20}\.txt",
        content1 in ".*{0,1000}",
        content2 in ".*{0,1000}"
    ) {
        let temp_dir = TempDir::new().unwrap();
        
        // Build nested path
        let mut full_path = temp_dir.path().to_path_buf();
        for part in &path_parts {
            full_path = full_path.join(part);
        }
        std::fs::create_dir_all(&full_path).unwrap();
        
        let file_path = full_path.join(&filename);
        
        // Create file with first content
        std::fs::write(&file_path, content1.as_bytes()).unwrap();
        let info1 = get_file_info(&file_path).unwrap();
        
        // Update file with different content
        std::fs::write(&file_path, content2.as_bytes()).unwrap();
        let info2 = get_file_info(&file_path).unwrap();
        
        // UUID should be the same (based on path, not content)
        prop_assert_eq!(info1.id, info2.id);
        
        // But size should be different (unless content happens to be same length)
        if content1.len() != content2.len() {
            prop_assert_ne!(info1.size, info2.size);
        }
        
        // Name should remain the same
        prop_assert_eq!(info1.name.clone(), info2.name);
        prop_assert_eq!(info1.name, filename);
    }
}

// Property test for large file handling
proptest! {
    #[test]
    fn test_large_file_handling(
        chunk_size in 1024usize..100_000,
        num_chunks in 1usize..100
    ) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");
        
        let total_size = chunk_size * num_chunks;
        
        // Create large file in chunks to avoid memory issues
        {
            let mut file = File::create(&file_path).unwrap();
            let chunk = vec![0x42u8; chunk_size];
            for _ in 0..num_chunks {
                file.write_all(&chunk).unwrap();
            }
        }
        
        let info = get_file_info(&file_path).unwrap();
        
        // Size should match exactly
        prop_assert_eq!(info.size, total_size as u64);
        
        // Size human should be reasonable
        prop_assert!(!info.size_human.is_empty());
        prop_assert!(info.size_human.ends_with("B"));
        
        // Should handle large files without panic
        prop_assert_eq!(info.name, "large_file.bin");
        prop_assert_eq!(info.mime_type, "application/octet-stream");
    }
} 