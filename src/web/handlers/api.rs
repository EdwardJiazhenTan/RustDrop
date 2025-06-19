use axum::{
    extract::{Path, State, Multipart},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{info, error};

use crate::core::models::{DeviceInfo, FileInfo};
use crate::discovery::ServiceDiscovery;
use crate::utils::file::{get_file_info, list_directory};

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "service": "rustdrop"
    }))
}

pub async fn get_device_info(
    State((_, device_info)): State<(PathBuf, DeviceInfo)>,
) -> Json<DeviceInfo> {
    Json(device_info)
}

pub async fn list_files(
    State((directory, _)): State<(PathBuf, DeviceInfo)>,
) -> Result<Json<Vec<FileInfo>>, StatusCode> {
    match list_directory(&directory) {
        Ok(files) => Ok(Json(files)),
        Err(e) => {
            error!("Failed to list directory: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn upload_file(
    State((directory, _)): State<(PathBuf, DeviceInfo)>,
    mut multipart: Multipart,
) -> Result<Json<FileInfo>, StatusCode> {
    info!("Upload request received");
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let file_name = field.file_name().ok_or_else(|| {
            error!("File name is missing from multipart field");
            StatusCode::BAD_REQUEST
        })?.to_string();
        
        info!("Processing file upload: {}", file_name);
        
        // Validate filename
        if file_name.is_empty() {
            error!("Empty filename provided");
            return Err(StatusCode::BAD_REQUEST);
        }
        
        let file_path = directory.join(&file_name);
        info!("File will be saved to: {:?}", file_path);
        
        // Create the file
        let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
            error!("Failed to create file {:?}: {}", file_path, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        // Write the file data
        let data = field.bytes().await.map_err(|e| {
            error!("Failed to read file data for {}: {}", file_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        info!("Received {} bytes for file {}", data.len(), file_name);
        
        file.write_all(&data).await.map_err(|e| {
            error!("Failed to write file data for {}: {}", file_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        // Ensure data is flushed to disk
        file.flush().await.map_err(|e| {
            error!("Failed to flush file {}: {}", file_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        file.sync_all().await.map_err(|e| {
            error!("Failed to sync file {}: {}", file_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        // Drop the file handle to ensure it's closed
        drop(file);
        
        // Get file info
        let file_info = get_file_info(&file_path).map_err(|e| {
            error!("Failed to get file info for {}: {}", file_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        info!("File uploaded successfully: {} ({}  bytes)", file_name, file_info.size);
        return Ok(Json(file_info));
    }
    
    error!("No file found in multipart request");
    Err(StatusCode::BAD_REQUEST)
}

pub async fn download_file(
    State((directory, _)): State<(PathBuf, DeviceInfo)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Find the file with the given ID
    let files = list_directory(&directory).map_err(|e| {
        error!("Failed to list directory: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let file = files.iter().find(|f| f.id.to_string() == id).ok_or_else(|| {
        error!("File not found: {}", id);
        StatusCode::NOT_FOUND
    })?;
    
    // Prepare headers
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        file.mime_type.parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file.name).parse().unwrap(),
    );
    
    // Read the file
    let file_data = tokio::fs::read(&file.path).await.map_err(|e| {
        error!("Failed to read file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    info!("File downloaded: {}", file.name);
    Ok((headers, file_data))
}

pub async fn discover_devices() -> Result<Json<Vec<DeviceInfo>>, StatusCode> {
    match ServiceDiscovery::discover().await {
        Ok(devices) => Ok(Json(devices)),
        Err(e) => {
            error!("Failed to discover devices: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle 404 errors for API routes
pub async fn api_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({
        "error": "API endpoint not found"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_device_info() -> DeviceInfo {
        DeviceInfo {
            id: Uuid::new_v4().to_string(),
            name: "test-device".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 8080,
            os: "linux".to_string(),
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        let Json(health_data) = response;

        assert_eq!(health_data["status"], "healthy");
        assert_eq!(health_data["service"], "rustdrop");
        assert_eq!(health_data["version"], env!("CARGO_PKG_VERSION"));
        assert!(health_data["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_get_device_info() {
        let temp_dir = TempDir::new().unwrap();
        let device_info = create_test_device_info();
        let state = (temp_dir.path().to_path_buf(), device_info.clone());

        let response = get_device_info(State(state)).await;
        let Json(returned_device) = response;

        assert_eq!(returned_device.id, device_info.id);
        assert_eq!(returned_device.name, device_info.name);
        assert_eq!(returned_device.ip, device_info.ip);
        assert_eq!(returned_device.port, device_info.port);
        assert_eq!(returned_device.os, device_info.os);
    }

    #[tokio::test]
    async fn test_list_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let device_info = create_test_device_info();
        let state = (temp_dir.path().to_path_buf(), device_info);

        let response = list_files(State(state)).await;
        assert!(response.is_ok());

        let Json(files) = response.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_list_files_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let device_info = create_test_device_info();

        // Create test files
        let file_names = vec!["test1.txt", "test2.txt", "test3.txt"];
        for name in &file_names {
            let file_path = temp_dir.path().join(name);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Test content for {}", name).unwrap();
        }

        let state = (temp_dir.path().to_path_buf(), device_info);
        let response = list_files(State(state)).await;
        assert!(response.is_ok());

        let Json(files) = response.unwrap();
        assert_eq!(files.len(), 3);

        // Check files are sorted by name
        assert_eq!(files[0].name, "test1.txt");
        assert_eq!(files[1].name, "test2.txt");
        assert_eq!(files[2].name, "test3.txt");

        // Check file properties
        for file in files {
            assert!(file.size > 0);
            assert_eq!(file.mime_type, "text/plain");
            assert!(temp_dir.path().join(&file.name).exists());
        }
    }

    #[tokio::test]
    async fn test_list_files_nonexistent_directory() {
        let device_info = create_test_device_info();
        let nonexistent_path = PathBuf::from("/nonexistent/directory");
        let state = (nonexistent_path, device_info);

        let response = list_files(State(state)).await;
        assert!(response.is_ok());

        let Json(files) = response.unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_health_check_response_format() {
        // Test that health check returns expected JSON structure
        tokio_test::block_on(async {
            let response = health_check().await;
            let Json(data) = response;

            // Check required fields exist
            assert!(data.get("status").is_some());
            assert!(data.get("timestamp").is_some());
            assert!(data.get("version").is_some());
            assert!(data.get("service").is_some());

            // Check field types
            assert!(data["status"].is_string());
            assert!(data["timestamp"].is_string());
            assert!(data["version"].is_string());
            assert!(data["service"].is_string());

            // Check specific values
            assert_eq!(data["status"], "healthy");
            assert_eq!(data["service"], "rustdrop");
        });
    }

    #[test]
    fn test_device_info_state_extraction() {
        tokio_test::block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let original_device = DeviceInfo {
                id: "test-id-123".to_string(),
                name: "test-device-name".to_string(),
                ip: "192.168.1.100".to_string(),
                port: 9999,
                os: "test-os".to_string(),
            };

            let state = (temp_dir.path().to_path_buf(), original_device.clone());
            let response = get_device_info(State(state)).await;
            let Json(extracted_device) = response;

            // Verify all fields are correctly extracted
            assert_eq!(extracted_device.id, original_device.id);
            assert_eq!(extracted_device.name, original_device.name);
            assert_eq!(extracted_device.ip, original_device.ip);
            assert_eq!(extracted_device.port, original_device.port);
            assert_eq!(extracted_device.os, original_device.os);
        });
    }

    #[test]
    fn test_file_listing_handles_mixed_content() {
        tokio_test::block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let device_info = create_test_device_info();

            // Create different types of files
            let file_path1 = temp_dir.path().join("text.txt");
            std::fs::write(&file_path1, "Hello, World!").unwrap();

            let file_path2 = temp_dir.path().join("data.json");
            std::fs::write(&file_path2, r#"{"key": "value"}"#).unwrap();

            let file_path3 = temp_dir.path().join("binary.bin");
            std::fs::write(&file_path3, &[0u8, 1, 2, 3, 255]).unwrap();

            // Create a subdirectory (should be ignored)
            std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

            let state = (temp_dir.path().to_path_buf(), device_info);
            let response = list_files(State(state)).await;
            assert!(response.is_ok());

            let Json(files) = response.unwrap();
            
            // Should only list files, not directories
            assert_eq!(files.len(), 3);

            // Check that files are sorted and have correct properties
            let file_names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
            assert_eq!(file_names, vec!["binary.bin", "data.json", "text.txt"]);

            // Check MIME types are detected correctly
            let json_file = files.iter().find(|f| f.name == "data.json").unwrap();
            assert_eq!(json_file.mime_type, "application/json");

            let txt_file = files.iter().find(|f| f.name == "text.txt").unwrap();
            assert_eq!(txt_file.mime_type, "text/plain");
        });
    }

    // Note: Testing upload_file and download_file would require more complex setup
    // with multipart form data and actual HTTP request/response handling.
    // These are better tested as integration tests.
}
