use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use std::process::{Child, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use reqwest;
use tokio;

const SERVER_STARTUP_WAIT: Duration = Duration::from_secs(3);
const SERVER_SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

struct TestServer {
    child: Child,
    port: u16,
    temp_dir: TempDir,
}

impl TestServer {
    async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let port = find_free_port().unwrap_or(8080);
        
        // Build the binary first to ensure it's available
        Command::cargo_bin("rustdrop")?
            .arg("--help")
            .assert()
            .success();

        let child = std::process::Command::new("cargo")
            .args(&["run", "--"])
            .args(&[
                "--directory", temp_dir.path().to_str().unwrap(),
                "--port", &port.to_string(),
                "--no-mdns",
                "--no-qr",
                "--no-browser"
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        // Wait for server to start
        thread::sleep(SERVER_STARTUP_WAIT);

        Ok(TestServer {
            child,
            port,
            temp_dir,
        })
    }

    fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    fn directory(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/api/health", self.base_url());
        
        for _ in 0..30 { // Try for 30 seconds
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        
        Err("Server failed to become ready".into())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        thread::sleep(SERVER_SHUTDOWN_WAIT);
    }
}

fn find_free_port() -> Option<u16> {
    use std::net::TcpListener;
    
    for port in 8080..9080 {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            drop(listener);
            return Some(port);
        }
    }
    None
}

#[tokio::test]
async fn test_cli_help_output() {
    let mut cmd = Command::cargo_bin("rustdrop").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cross-platform file transfer"))
        .stdout(predicate::str::contains("--directory"))
        .stdout(predicate::str::contains("--port"));
}

#[tokio::test]
async fn test_cli_version_output() {
    let mut cmd = Command::cargo_bin("rustdrop").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[tokio::test]
async fn test_cli_invalid_arguments() {
    // Test invalid port
    let mut cmd = Command::cargo_bin("rustdrop").unwrap();
    cmd.arg("--port").arg("99999")
        .assert()
        .failure();

    // Test invalid directory
    let mut cmd = Command::cargo_bin("rustdrop").unwrap();
    cmd.arg("--directory").arg("/nonexistent/directory")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_server_lifecycle() {
    let server = TestServer::start().await.unwrap();
    
    // Wait for server to be ready
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    
    // Test health endpoint
    let health_url = format!("{}/api/health", server.base_url());
    let response = client.get(&health_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let health_data: serde_json::Value = response.json().await.unwrap();
    assert_eq!(health_data["status"], "healthy");
    assert_eq!(health_data["service"], "rustdrop");
}

#[tokio::test]
async fn test_complete_file_workflow() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    
    // 1. Create test files in the server directory
    let test_files = vec![
        ("document.txt", "This is a test document."),
        ("data.json", r#"{"test": true, "number": 42}"#),
        ("image.png", "fake-png-data"), // Simulated binary
    ];
    
    for (filename, content) in &test_files {
        let file_path = server.directory().join(filename);
        fs::write(&file_path, content).unwrap();
    }
    
    // 2. List files via API
    let files_url = format!("{}/api/files", server.base_url());
    let response = client.get(&files_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let files: serde_json::Value = response.json().await.unwrap();
    assert!(files.is_array());
    let files_array = files.as_array().unwrap();
    assert_eq!(files_array.len(), 3);
    
    // 3. Get device info
    let device_url = format!("{}/api/device", server.base_url());
    let response = client.get(&device_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let device_info: serde_json::Value = response.json().await.unwrap();
    assert!(device_info["id"].is_string());
    assert!(device_info["name"].is_string());
    assert_eq!(device_info["port"], server.port);
    
    // 4. Download each file and verify content
    for (expected_filename, expected_content) in &test_files {
        // Find the file in the list
        let file_entry = files_array.iter()
            .find(|f| f["name"] == *expected_filename)
            .unwrap();
        
        let file_id = file_entry["id"].as_str().unwrap();
        
        // Download the file
        let download_url = format!("{}/api/files/{}", server.base_url(), file_id);
        let response = client.get(&download_url).send().await.unwrap();
        assert!(response.status().is_success());
        
        let content = response.text().await.unwrap();
        assert_eq!(&content, expected_content);
    }
}

#[tokio::test]
async fn test_concurrent_requests() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    // Create multiple test files
    for i in 0..10 {
        let filename = format!("file_{}.txt", i);
        let content = format!("Content of file {}", i);
        let file_path = server.directory().join(&filename);
        fs::write(&file_path, &content).unwrap();
    }
    
    let client = reqwest::Client::new();
    let health_url = format!("{}/api/health", server.base_url());
    let files_url = format!("{}/api/files", server.base_url());
    let device_url = format!("{}/api/device", server.base_url());
    
    // Make concurrent requests
    let mut handles = vec![];
    
    for _ in 0..20 {
        let client = client.clone();
        let health_url = health_url.clone();
        let files_url = files_url.clone();
        let device_url = device_url.clone();
        
        let handle = tokio::spawn(async move {
            // Test health endpoint
            let response = client.get(&health_url).send().await.unwrap();
            assert!(response.status().is_success());
            
            // Test files endpoint
            let response = client.get(&files_url).send().await.unwrap();
            assert!(response.status().is_success());
            
            // Test device endpoint
            let response = client.get(&device_url).send().await.unwrap();
            assert!(response.status().is_success());
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_file_changes_during_runtime() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    let files_url = format!("{}/api/files", server.base_url());
    
    // Initially no files
    let response = client.get(&files_url).send().await.unwrap();
    let files: serde_json::Value = response.json().await.unwrap();
    assert_eq!(files.as_array().unwrap().len(), 0);
    
    // Add a file
    let file_path = server.directory().join("new_file.txt");
    fs::write(&file_path, "New file content").unwrap();
    
    // Should see the new file
    let response = client.get(&files_url).send().await.unwrap();
    let files: serde_json::Value = response.json().await.unwrap();
    assert_eq!(files.as_array().unwrap().len(), 1);
    assert_eq!(files[0]["name"], "new_file.txt");
    
    // Modify the file
    fs::write(&file_path, "Modified content").unwrap();
    
    // File list should still show the file (content changed but not visible in listing)
    let response = client.get(&files_url).send().await.unwrap();
    let files: serde_json::Value = response.json().await.unwrap();
    assert_eq!(files.as_array().unwrap().len(), 1);
    
    // Remove the file
    fs::remove_file(&file_path).unwrap();
    
    // Should no longer see the file
    let response = client.get(&files_url).send().await.unwrap();
    let files: serde_json::Value = response.json().await.unwrap();
    assert_eq!(files.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_large_file_handling() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    // Create a large file (1MB)
    let large_content = "x".repeat(1024 * 1024);
    let file_path = server.directory().join("large_file.bin");
    fs::write(&file_path, &large_content).unwrap();
    
    let client = reqwest::Client::new();
    
    // List files should handle large file
    let files_url = format!("{}/api/files", server.base_url());
    let response = client.get(&files_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let files: serde_json::Value = response.json().await.unwrap();
    assert_eq!(files.as_array().unwrap().len(), 1);
    
    let large_file = &files[0];
    assert_eq!(large_file["name"], "large_file.bin");
    assert_eq!(large_file["size"], 1024 * 1024);
    assert!(large_file["size_human"].as_str().unwrap().contains("MiB") || 
             large_file["size_human"].as_str().unwrap().contains("MB"));
    
    // Download large file should work
    let file_id = large_file["id"].as_str().unwrap();
    let download_url = format!("{}/api/files/{}", server.base_url(), file_id);
    let response = client.get(&download_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let downloaded_content = response.text().await.unwrap();
    assert_eq!(downloaded_content.len(), large_content.len());
    assert_eq!(downloaded_content, large_content);
}

#[tokio::test]
async fn test_directory_with_many_files() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    // Create many files
    for i in 0..100 {
        let filename = format!("file_{:03}.txt", i);
        let content = format!("Content of file {}", i);
        let file_path = server.directory().join(&filename);
        fs::write(&file_path, &content).unwrap();
    }
    
    let client = reqwest::Client::new();
    let files_url = format!("{}/api/files", server.base_url());
    
    // Should handle listing many files
    let response = client.get(&files_url).send().await.unwrap();
    assert!(response.status().is_success());
    
    let files: serde_json::Value = response.json().await.unwrap();
    let files_array = files.as_array().unwrap();
    assert_eq!(files_array.len(), 100);
    
    // Files should be sorted
    for i in 0..99 {
        let current_name = files_array[i]["name"].as_str().unwrap();
        let next_name = files_array[i + 1]["name"].as_str().unwrap();
        assert!(current_name <= next_name, "Files should be sorted");
    }
}

#[tokio::test]
async fn test_server_error_recovery() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    
    // Make requests to endpoints that should handle errors gracefully
    let test_urls = vec![
        format!("{}/api/files/invalid-uuid", server.base_url()),
        format!("{}/api/files/00000000-0000-0000-0000-000000000000", server.base_url()),
        format!("{}/api/nonexistent", server.base_url()),
        format!("{}/static/nonexistent.html", server.base_url()),
    ];
    
    for url in test_urls {
        let response = client.get(&url).send().await.unwrap();
        // Should not crash the server, should return appropriate error codes
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }
    
    // Server should still be responsive after errors
    let health_url = format!("{}/api/health", server.base_url());
    let response = client.get(&health_url).send().await.unwrap();
    assert!(response.status().is_success());
}

#[tokio::test] 
async fn test_cors_in_practice() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    
    // Test actual CORS request
    let response = client
        .get(&format!("{}/api/health", server.base_url()))
        .header("Origin", "http://localhost:3000")
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    
    // Check CORS headers are present
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_static_file_serving() {
    let server = TestServer::start().await.unwrap();
    server.wait_for_ready().await.unwrap();
    
    let client = reqwest::Client::new();
    
    // Test root path (should serve fallback)
    let response = client.get(&server.base_url()).send().await.unwrap();
    assert!(response.status().is_success());
    
    // Response should be HTML content (fallback index.html)
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html") ||
            content_type.to_str().unwrap().contains("text/plain"));
} 