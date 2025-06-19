use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use rustdrop::web::routes::create_routes;
use rustdrop::core::models::DeviceInfo;
use rustdrop::{AppConfig, get_file_info, list_directory};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use tower::util::ServiceExt;
use tower_http::cors::{Any, CorsLayer};

// Helper function to create test app
fn create_test_app(temp_dir: &TempDir) -> Router {
    let device_info = DeviceInfo::new(8080);
    let directory = temp_dir.path().to_path_buf();
    let max_file_size = 10 * 1024 * 1024; // 10MB
    
    // Add CORS layer like in the actual server
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    create_routes(directory, device_info, max_file_size)
        .layer(cors)
}

#[tokio::test]
async fn test_health_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health_data: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(health_data["status"], "healthy");
    assert_eq!(health_data["service"], "rustdrop");
    assert!(health_data["timestamp"].is_string());
    assert!(health_data["version"].is_string());
}

#[tokio::test]
async fn test_device_info_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/device")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let device_info: DeviceInfo = serde_json::from_slice(&body).unwrap();

    assert!(!device_info.id.is_empty());
    assert!(!device_info.name.is_empty());
    assert!(!device_info.ip.is_empty());
    assert_eq!(device_info.port, 8080);
    assert!(!device_info.os.is_empty());
}

#[tokio::test]
async fn test_list_files_empty() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/files")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let files: Vec<Value> = serde_json::from_slice(&body).unwrap();

    assert!(files.is_empty());
}

#[tokio::test]
async fn test_list_files_with_content() {
    let temp_dir = TempDir::new().unwrap();

    // Create test files
    let test_files = vec![
        ("document.txt", "Hello, World!"),
        ("data.json", r#"{"key": "value"}"#),
        ("readme.md", "# Test Document"),
    ];

    for (filename, content) in &test_files {
        let file_path = temp_dir.path().join(filename);
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
    }

    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/files")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let files: Vec<Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(files.len(), 3);

    // Check that files are sorted
    let names: Vec<String> = files
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["data.json", "document.txt", "readme.md"]);

    // Check file properties
    for file in files {
        assert!(file["id"].is_string());
        assert!(file["name"].is_string());
        assert!(file["size"].is_number());
        assert!(file["size_human"].is_string());
        assert!(file["mime_type"].is_string());
        assert!(file["modified"].is_string());
    }
}

#[tokio::test]
async fn test_download_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let fake_uuid = "123e4567-e89b-12d3-a456-426614174000";
    let request = Request::builder()
        .uri(&format!("/api/files/{}", fake_uuid))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_download_existing_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file
    let file_content = "This is test content for download";
    let file_path = temp_dir.path().join("download_test.txt");
    std::fs::write(&file_path, file_content).unwrap();

    // Get file info to get its UUID
    let file_info = get_file_info(&file_path).unwrap();
    let file_id = file_info.id.to_string();

    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri(&format!("/api/files/{}", file_id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Check headers
    let headers = response.headers();
    assert!(headers.contains_key("content-type"));
    assert!(headers.contains_key("content-disposition"));

    let content_disposition = headers.get("content-disposition").unwrap();
    assert!(content_disposition
        .to_str()
        .unwrap()
        .contains("download_test.txt"));

    // Check content
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let downloaded_content = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(downloaded_content, file_content);
}

#[tokio::test]
async fn test_discover_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/discover")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Note: This might return an error in CI environment where mDNS isn't available
    // We just check that the endpoint responds properly
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_invalid_endpoints() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let test_cases = vec![
        "/api/nonexistent",
        "/api/files/invalid-uuid-format",
        "/api/invalid",
    ];

    for uri in test_cases {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should return 404 for invalid endpoints
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_cors_headers() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let request = Request::builder()
        .uri("/api/health")
        .header("origin", "http://localhost:3000")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    // CORS headers should be present
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_static_file_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Request root path should serve index.html (fallback)
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 200 OK (serving the fallback)
    assert_eq!(response.status(), StatusCode::OK);
}

// Configuration Tests
#[test]
fn test_config_defaults() {
    let config = AppConfig::default();

    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.max_file_size, 1024 * 1024 * 1024);
    assert!(config.discovery.enabled);
    assert!(config.ui.qr_code);
    assert!(!config.ui.open_browser);
}

#[test]
fn test_config_loading_from_toml() {
    let toml_content = r#"
        [server]
        port = 9000
        host = "127.0.0.1"
        max_file_size = 500000000

        [files]
        expiry_hours = 48

        [discovery]
        enabled = false

        [ui]
        qr_code = false
        open_browser = true
    "#;

    let config = AppConfig::from_toml(toml_content).unwrap();

    assert_eq!(config.server.port, 9000);
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.max_file_size, 500000000);
    assert_eq!(config.files.expiry_hours, Some(48));
    assert!(!config.discovery.enabled);
    assert!(!config.ui.qr_code);
    assert!(config.ui.open_browser);
}

// File Operations Tests
#[test]
fn test_file_operations_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Test directory listing on empty directory
    let files = list_directory(temp_dir.path()).unwrap();
    assert!(files.is_empty());

    // Create test files with different types
    let test_cases = vec![
        ("text.txt", "Hello, World!", "text/plain"),
        ("data.json", r#"{"test": true}"#, "application/json"),
        ("style.css", "body { margin: 0; }", "text/css"),
        ("script.js", "console.log('test');", "text/javascript"),
    ];

    for (filename, content, expected_mime) in &test_cases {
        let file_path = temp_dir.path().join(filename);
        std::fs::write(&file_path, content).unwrap();

        // Test individual file info
        let file_info = get_file_info(&file_path).unwrap();
        assert_eq!(file_info.name, *filename);
        assert_eq!(file_info.size, content.len() as u64);
        assert_eq!(file_info.mime_type, *expected_mime);
        assert!(file_info.size_human.ends_with("B"));
    }

    // Test directory listing with files
    let files = list_directory(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 4);

    // Check sorting
    let names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    assert_eq!(names, vec!["data.json", "script.js", "style.css", "text.txt"]);

    // Test UUID consistency
    let file_path = temp_dir.path().join("text.txt");
    let info1 = get_file_info(&file_path).unwrap();
    let info2 = get_file_info(&file_path).unwrap();
    assert_eq!(info1.id, info2.id);
}

// Network and Port Tests
#[test]
fn test_port_availability() {
    use rustdrop::utils::network::{find_available_port, is_port_available};

    // Test port availability check
    let high_port = 65000;
    assert!(is_port_available(high_port));

    // Test finding available port in range
    let available_port = find_available_port(60000, 60100);
    assert!(available_port.is_some());

    let port = available_port.unwrap();
    assert!(port >= 60000 && port <= 60100);
    assert!(is_port_available(port));
}

// Device Info Tests
#[test]
fn test_device_info_creation() {
    let device1 = DeviceInfo::new(8080);
    let device2 = DeviceInfo::new(8080);

    // Different instances should have different IDs
    assert_ne!(device1.id, device2.id);

    // But same network info (on same machine)
    assert_eq!(device1.ip, device2.ip);
    assert_eq!(device1.os, device2.os);
    assert_eq!(device1.port, device2.port);

    // URL generation
    let url = device1.url();
    assert!(url.starts_with("http://"));
    assert!(url.contains("8080"));
    assert!(url.contains(&device1.ip));
}

// Error Handling Tests
#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Test malformed requests
    let request = Request::builder()
        .uri("/api/files/%invalid-uuid%")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_concurrent_file_operations() {
    use std::thread;

    let temp_dir = TempDir::new().unwrap();

    // Create files concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let temp_path = temp_dir.path().to_path_buf();
            thread::spawn(move || {
                let file_path = temp_path.join(format!("concurrent_{}.txt", i));
                std::fs::write(&file_path, format!("Content {}", i)).unwrap();
                get_file_info(&file_path).unwrap()
            })
        })
        .collect();

    let file_infos: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert_eq!(file_infos.len(), 5);

    // Check all files have unique IDs
    let mut ids = std::collections::HashSet::new();
    for info in file_infos {
        assert!(ids.insert(info.id)); // Returns false if already exists
    }

    // List directory to ensure all files are visible
    let files = list_directory(temp_dir.path()).unwrap();
    assert_eq!(files.len(), 5);
} 