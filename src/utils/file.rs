use anyhow::Result;
use chrono::{DateTime, Utc};
use humansize::{format_size, BINARY};
use mime_guess::from_path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use uuid::Uuid;

use crate::core::models::FileInfo;

pub fn get_file_info(path: &Path) -> Result<FileInfo> {
    let metadata = std::fs::metadata(path)?;
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Generate deterministic UUID based on file path
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    let hash = hasher.finish();
    
    // Convert hash to UUID bytes
    let uuid_bytes = [
        (hash >> 56) as u8,
        (hash >> 48) as u8,
        (hash >> 40) as u8,
        (hash >> 32) as u8,
        (hash >> 24) as u8,
        (hash >> 16) as u8,
        (hash >> 8) as u8,
        hash as u8,
        0, 0, 0, 0, 0, 0, 0, 0, // Pad to 16 bytes
    ];
    
    let id = Uuid::from_bytes(uuid_bytes);
    let size = metadata.len();
    let size_human = format_size(size, BINARY);
    let modified = DateTime::<Utc>::from(metadata.modified()?);
    let mime_type = from_path(path).first_or_octet_stream().to_string();
    
    Ok(FileInfo {
        id,
        name,
        size,
        size_human,
        modified,
        mime_type,
        path: path.to_path_buf(),
    })
}

pub fn list_directory(dir: &Path) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    
    if !dir.exists() {
        return Ok(files);
    }
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Ok(file_info) = get_file_info(&path) {
                files.push(file_info);
            }
        }
    }
    
    // Sort by name for consistent ordering
    files.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(files)
}

pub fn format_file_size(size: u64) -> String {
    format_size(size, BINARY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_get_file_info_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();
        
        // Test getting file info
        let file_info = get_file_info(&file_path).unwrap();
        
        assert_eq!(file_info.name, "test.txt");
        assert!(file_info.size > 0);
        assert_eq!(file_info.mime_type, "text/plain");
        assert_eq!(file_info.path, file_path);
    }

    #[test]
    fn test_deterministic_uuid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("uuid_test.txt");
        
        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "UUID test").unwrap();
        
        // Get file info multiple times
        let info1 = get_file_info(&file_path).unwrap();
        let info2 = get_file_info(&file_path).unwrap();
        
        // UUIDs should be identical
        assert_eq!(info1.id, info2.id);
    }

    #[test]
    fn test_different_files_different_uuids() {
        let temp_dir = TempDir::new().unwrap();
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");
        
        // Create two different files
        let mut file1 = File::create(&file1_path).unwrap();
        writeln!(file1, "File 1").unwrap();
        
        let mut file2 = File::create(&file2_path).unwrap();
        writeln!(file2, "File 2").unwrap();
        
        // Get file info
        let info1 = get_file_info(&file1_path).unwrap();
        let info2 = get_file_info(&file2_path).unwrap();
        
        // UUIDs should be different
        assert_ne!(info1.id, info2.id);
    }

    #[test]
    fn test_mime_type_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test various file types
        let test_cases = vec![
            ("test.txt", "text/plain"),
            ("test.html", "text/html"),
            ("test.json", "application/json"),
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
        ];
        
        for (filename, expected_mime) in test_cases {
            let file_path = temp_dir.path().join(filename);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "test content").unwrap();
            
            let file_info = get_file_info(&file_path).unwrap();
            assert_eq!(file_info.mime_type, expected_mime);
        }
    }

    #[test]
    fn test_file_size_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("size_test.txt");
        
        // Create file with specific content
        let content = "A".repeat(100); // 100 bytes
        std::fs::write(&file_path, &content).unwrap();
        
        let file_info = get_file_info(&file_path).unwrap();
        assert_eq!(file_info.size, 100);
        assert_eq!(file_info.size_human, "100 B");
    }

    #[test]
    fn test_list_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let files = list_directory(temp_dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_list_nonexistent_directory() {
        let nonexistent_path = Path::new("/nonexistent/directory");
        let files = list_directory(nonexistent_path).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_list_directory_with_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create multiple test files
        let filenames = vec!["zebra.txt", "alpha.txt", "beta.txt"];
        for filename in &filenames {
            let file_path = temp_dir.path().join(filename);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Content of {}", filename).unwrap();
        }
        
        let files = list_directory(temp_dir.path()).unwrap();
        
        assert_eq!(files.len(), 3);
        
        // Check files are sorted by name
        assert_eq!(files[0].name, "alpha.txt");
        assert_eq!(files[1].name, "beta.txt");
        assert_eq!(files[2].name, "zebra.txt");
    }

    #[test]
    fn test_list_directory_ignores_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a file and a subdirectory
        let file_path = temp_dir.path().join("file.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test content").unwrap();
        
        let subdir_path = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir_path).unwrap();
        
        let files = list_directory(temp_dir.path()).unwrap();
        
        // Should only include the file, not the subdirectory
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "file.txt");
    }

    #[test]
    fn test_large_file_size_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        
        // Create a larger file (1KB)
        let content = "X".repeat(1024);
        std::fs::write(&file_path, &content).unwrap();
        
        let file_info = get_file_info(&file_path).unwrap();
        assert_eq!(file_info.size, 1024);
        assert_eq!(file_info.size_human, "1 KiB");
    }

    #[test]
    fn test_file_modification_time() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("time_test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test").unwrap();
        
        let file_info = get_file_info(&file_path).unwrap();
        
        // Modification time should be recent (within last minute)
        let now = Utc::now();
        let diff = now.signed_duration_since(file_info.modified);
        assert!(diff.num_seconds() < 60);
    }
}
