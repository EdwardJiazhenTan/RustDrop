use axum::{
    Router,
    routing::get,
    extract::DefaultBodyLimit,
};
use std::path::PathBuf;
use tower_http::services::ServeDir;

use crate::core::models::DeviceInfo;
use crate::web::handlers::{
    api::{
        health_check,
        get_device_info,
        list_files,
        upload_file,
        download_file,
        discover_devices,
        api_not_found,
    },
    static_files::serve_index,
};

pub fn create_routes(directory: PathBuf, device_info: DeviceInfo, max_file_size: u64) -> Router {
    // API routes
    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/device", get(get_device_info))
        .route("/files", get(list_files).post(upload_file))
        .route("/files/:id", get(download_file))
        .route("/discover", get(discover_devices))
        .fallback(api_not_found)
        .with_state((directory.clone(), device_info));
    
    // Static file serving for the web UI
    let static_routes = Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback(serve_index);
    
    // Combine routes
    Router::new()
        .nest("/api", api_routes)
        .merge(static_routes)
        .layer(DefaultBodyLimit::max(max_file_size as usize))
}
