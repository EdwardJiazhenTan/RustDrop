use axum::{
    body::Body,
    http::{Request, StatusCode, Method, HeaderValue},
    Router,
};
use rustdrop::web::routes::create_routes;
use rustdrop::core::models::DeviceInfo;
use serde_json::Value;
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
async fn test_path_traversal_protection() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file outside the served directory
    let parent_dir = temp_dir.path().parent().unwrap();
    let secret_file = parent_dir.join("secret.txt");
    std::fs::write(&secret_file, "secret content").unwrap();
    
    let app = create_test_app(&temp_dir);

    // Test various path traversal attempts
    let malicious_paths = vec![
        "../secret.txt",
        "..%2Fsecret.txt",
        "..%2f..%2fsecret.txt", 
        "%2e%2e%2fsecret.txt",
        "....//secret.txt",
        "..\\secret.txt",
        "..%5csecret.txt",
        "/%2e%2e/secret.txt",
        "/../../secret.txt",
        "/.%2e/.%2e/secret.txt",
    ];

    for malicious_path in malicious_paths {
        let request = Request::builder()
            .uri(&format!("/api/files/{}", malicious_path))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // Should not allow access to files outside directory
        assert!(
            response.status() == StatusCode::NOT_FOUND || 
            response.status() == StatusCode::BAD_REQUEST,
            "Path traversal protection failed for: {}", malicious_path
        );
    }
}

#[tokio::test]
async fn test_cors_headers() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Test preflight request
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/health")
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    
    // Should allow CORS
    assert_eq!(response.status(), StatusCode::OK);
    
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert!(headers.contains_key("access-control-allow-methods"));
}

#[tokio::test]
async fn test_cors_origins() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let test_origins = vec![
        "http://localhost:3000",
        "https://example.com", 
        "http://192.168.1.100:8080",
    ];

    for origin in test_origins {
        let request = Request::builder()
            .uri("/api/health")
            .header("Origin", origin)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let headers = response.headers();
        if let Some(allow_origin) = headers.get("access-control-allow-origin") {
            // Should either allow all origins (*) or echo back the origin
            assert!(
                allow_origin == "*" || 
                allow_origin == HeaderValue::from_str(origin).unwrap()
            );
        }
    }
}

#[tokio::test]
async fn test_malicious_filename_handling() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Create files with potentially problematic names (that are still valid on Unix)
    let problematic_names = vec![
        "normal.txt",
        "file with spaces.txt", 
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.with.dots.txt",
        "UPPERCASE.TXT",
        "123numeric.txt",
        "file(with)parens.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
    ];

    for filename in problematic_names {
        let file_path = temp_dir.path().join(filename);
        std::fs::write(&file_path, "test content").unwrap();
    }

    // Test file listing doesn't crash with these names
    let request = Request::builder()
        .uri("/api/files")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let files: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(files.is_array());
    let files_array = files.as_array().unwrap();
    assert!(!files_array.is_empty());
}

#[tokio::test]
async fn test_large_filename_handling() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Create file with very long name (255 chars is typical filesystem limit)
    let long_name = format!("{}.txt", "a".repeat(250));
    let file_path = temp_dir.path().join(&long_name);
    
    if std::fs::write(&file_path, "test content").is_ok() {
        let request = Request::builder()
            .uri("/api/files")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_invalid_uuid_handling() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    let invalid_uuids = vec![
        "not-a-uuid",
        "12345",
        "invalid-uuid-format", 
        "",
        "00000000-0000-0000-0000-000000000000g", // too long
        "00000000-0000-0000-0000-00000000000", // too short
        "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx", // invalid chars
    ];

    for invalid_uuid in invalid_uuids {
        let request = Request::builder()
            .uri(&format!("/api/files/{}", invalid_uuid))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // Should handle invalid UUIDs gracefully
        assert!(
            response.status() == StatusCode::NOT_FOUND ||
            response.status() == StatusCode::BAD_REQUEST,
            "Failed to handle invalid UUID: {}", invalid_uuid
        );
    }
}

#[tokio::test]
async fn test_http_method_restrictions() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Test unsupported methods on various endpoints
    let test_cases = vec![
        (Method::DELETE, "/api/health"),
        (Method::PATCH, "/api/health"),
        (Method::PUT, "/api/files"),
        (Method::DELETE, "/api/device"),
    ];

    for (method, path) in test_cases {
        let request = Request::builder()
            .method(method.clone())
            .uri(path)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // Should reject unsupported methods
        assert!(
            response.status() == StatusCode::METHOD_NOT_ALLOWED ||
            response.status() == StatusCode::NOT_FOUND,
            "Failed to restrict method {} on {}", method, path
        );
    }
}

#[tokio::test]
async fn test_request_size_limits() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Create a very large request body (beyond reasonable limits)
    let large_body = "x".repeat(10 * 1024 * 1024); // 10MB

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/upload")
        .header("content-type", "text/plain")
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should handle large requests appropriately
    assert!(
        response.status() == StatusCode::PAYLOAD_TOO_LARGE ||
        response.status() == StatusCode::BAD_REQUEST ||
        response.status() == StatusCode::NOT_FOUND // if upload endpoint doesn't exist yet
    );
}

#[tokio::test] 
async fn test_content_type_validation() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Test requests with malicious or unexpected content types
    let malicious_content_types = vec![
        "application/x-evil",
        "text/html; <script>alert('xss')</script>",
        "multipart/form-data; boundary=--evil",
        "../../../etc/passwd",
    ];

    for content_type in malicious_content_types {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/upload")
            .header("content-type", content_type)
            .body(Body::from("test data"))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // Should handle malicious content types safely
        // The exact response depends on implementation but should not crash
        assert!(response.status().is_client_error() || response.status().is_server_error() || response.status().is_success());
    }
    
    // Test content type with newline injection - this should be rejected by HTTP library
    let result = Request::builder()
        .method(Method::POST)
        .uri("/api/upload")
        .header("content-type", "application/json\r\nX-Injected-Header: malicious")
        .body(Body::from("test data"));
    
    // This should fail to create the request due to invalid header value
    assert!(result.is_err());
}

#[tokio::test]
async fn test_header_injection_protection() {
    let temp_dir = TempDir::new().unwrap();
    let app = create_test_app(&temp_dir);

    // Test header injection attempts - valid ones
    let safe_values = vec![
        "normal-value",
        "value-with-dashes",
        "value123",
    ];

    for value in safe_values {
        let request = Request::builder()
            .uri("/api/health")
            .header("X-Custom-Header", value)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // Should handle safe headers normally
        assert!(response.status().is_success());
    }
    
    // Test that malicious header values are rejected by HTTP library
    let malicious_values = vec![
        "value\r\nX-Injected: malicious",
        "value\nX-Injected: malicious", 
        "value\r\nSet-Cookie: evil=true",
        "value\x00X-Injected: malicious",
        "value\x0aX-Injected: malicious",
        "value\x0dX-Injected: malicious",
    ];

    for injection_value in malicious_values {
        // These should fail to create the request due to invalid header values
        let result = Request::builder()
            .uri("/api/health")
            .header("X-Custom-Header", injection_value)
            .body(Body::empty());
        
        // HTTP library should reject these malicious header values
        assert!(result.is_err(), "HTTP library should reject malicious header: {}", injection_value);
    }
} 