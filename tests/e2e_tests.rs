use anyhow::Result;
use serial_test::serial;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

const SERVER_STARTUP_WAIT: Duration = Duration::from_millis(3000);
const SERVER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);

fn find_available_port() -> u16 {
    use std::net::TcpListener;
    
    // Try ports starting from 8080
    for port in 8080..9080 {
        if let Ok(_) = TcpListener::bind(("127.0.0.1", port)) {
            return port;
        }
    }
    8080 // fallback
}

struct TestServer {
    child: Child,
    port: u16,
    temp_dir: TempDir,
}

impl TestServer {
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let port = find_available_port();
        
        // Build the binary first if needed
        let output = Command::new("cargo")
            .args(&["build", "--bin", "rustdrop"])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to build rustdrop binary"));
        }
        
        // Start the server
        let child = Command::new("./target/debug/rustdrop")
            .args(&[
                "--port", &port.to_string(),
                "--directory", temp_dir.path().to_str().unwrap(),
                "--no-mdns",
                "--no-qr"
            ])
            .spawn()?;
        
        // Wait for server to start
        sleep(SERVER_STARTUP_WAIT).await;
        
        let server = Self {
            child,
            port,
            temp_dir,
        };
        
        // Wait for server to be ready
        server.wait_for_ready().await?;
        
        Ok(server)
    }
    
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    
    async fn wait_for_ready(&self) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
            
        for attempt in 0..20 {
            match client.get(&format!("{}/api/health", self.url())).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(());
                    }
                }
                Err(e) => {
                    // Check if it's a connection error (server not ready yet)
                    if e.is_connect() || e.is_timeout() {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    // For other errors, try a few more times
                    if attempt < 15 {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
        
        // Try fallback endpoint
        for _ in 0..5 {
            match client.get(&format!("{}/", self.url())).send().await {
                Ok(response) => {
                    if response.status().is_success() || response.status() == 404 {
                        return Ok(());
                    }
                }
                Err(_) => {
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
        
        Err(anyhow::anyhow!("Server failed to become ready after {} attempts", 25))
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
#[serial]
async fn test_server_starts() -> Result<()> {
    let _server = TestServer::new().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_api_endpoints() -> Result<()> {
    let server = TestServer::new().await?;
    let client = reqwest::Client::new();
    
    // Test health endpoint
    let response = timeout(
        SERVER_RESPONSE_TIMEOUT,
        client.get(&format!("{}/api/health", server.url())).send()
    ).await??;
    assert!(response.status().is_success());
    
    // Test device info endpoint
    let response = timeout(
        SERVER_RESPONSE_TIMEOUT,
        client.get(&format!("{}/api/device", server.url())).send()
    ).await??;
    assert!(response.status().is_success());
    
    // Test files endpoint
    let response = timeout(
        SERVER_RESPONSE_TIMEOUT,
        client.get(&format!("{}/api/files", server.url())).send()
    ).await??;
    assert!(response.status().is_success());
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_file_upload_and_download() -> Result<()> {
    let server = TestServer::new().await?;
    let client = reqwest::Client::new();
    
    // Create a test file in the server directory
    let test_file_path = server.temp_dir.path().join("test.txt");
    std::fs::write(&test_file_path, "Hello, World!")?;
    
    // Give the server a moment to detect the file
    sleep(Duration::from_millis(500)).await;
    
    // Test file listing first
    let response = timeout(
        SERVER_RESPONSE_TIMEOUT,
        client.get(&format!("{}/api/files", server.url())).send()
    ).await??;
    assert!(response.status().is_success());
    
    let files: serde_json::Value = response.json().await?;
    println!("Files response: {}", files);
    let files_array = files.as_array().unwrap();
    assert!(files_array.len() > 0);
    
    // Get the file ID from the response
    let file_id = files_array[0]["id"].as_str().unwrap();
    
    // Test file download using the file ID
    let response = timeout(
        SERVER_RESPONSE_TIMEOUT,
        client.get(&format!("{}/api/files/{}", server.url(), file_id)).send()
    ).await??;
    
    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        panic!("File download failed with status {}: {}", status, error_text);
    }
    
    let content = response.text().await?;
    assert_eq!(content, "Hello, World!");
    
    Ok(())
} 