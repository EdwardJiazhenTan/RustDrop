use rustdrop::core::models::DeviceInfo;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use rustdrop::utils::file::{get_file_info, list_directory};
use rustdrop::utils::network::{find_available_port, is_port_available};
use uuid::Uuid;

#[test]
fn test_concurrent_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let num_threads = 10;
    let files_per_thread = 50;
    
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];
    
    for _thread_id in 0..num_threads {
        let barrier = Arc::clone(&barrier);
        let temp_dir_path = temp_dir.path().to_path_buf();
        
        let handle = thread::spawn(move || {
            // Wait for all threads to be ready
            barrier.wait();
            
            let start_time = Instant::now();
            
            // Create files
            for file_id in 0..files_per_thread {
                let filename = format!("thread_{}_file_{}.txt", _thread_id, file_id);
                let content = format!("Content from thread {} file {}", _thread_id, file_id);
                let file_path = temp_dir_path.join(&filename);
                
                fs::write(&file_path, &content).unwrap();
                
                // Immediately try to get file info
                let file_info = get_file_info(&file_path).unwrap();
                assert_eq!(file_info.name, filename);
                assert_eq!(file_info.size, content.len() as u64);
            }
            
            let creation_time = start_time.elapsed();
            
            // List all files in directory (this will include files from other threads)
            let files = list_directory(&temp_dir_path).unwrap();
            
            let listing_time = start_time.elapsed();
            
            (creation_time, listing_time, files.len())
        });
        
        handles.push(handle);
    }
    
    // Collect results
    let mut total_creation_time = Duration::ZERO;
    let mut total_listing_time = Duration::ZERO;
    let mut final_file_count = 0;
    
    for handle in handles {
        let (creation_time, listing_time, file_count) = handle.join().unwrap();
        total_creation_time += creation_time;
        total_listing_time += listing_time;
        final_file_count = file_count; // All threads should see the same count eventually
    }
    
    // Verify results
    let expected_total_files = num_threads * files_per_thread;
    assert_eq!(final_file_count, expected_total_files);
    
    println!("Stress Test Results:");
    println!("- Threads: {}", num_threads);
    println!("- Files per thread: {}", files_per_thread);
    println!("- Total files: {}", expected_total_files);
    println!("- Average creation time per thread: {:?}", total_creation_time / num_threads as u32);
    println!("- Average listing time per thread: {:?}", total_listing_time / num_threads as u32);
}

#[test]
fn test_large_directory_performance() {
    let temp_dir = TempDir::new().unwrap();
    let file_counts = vec![100, 500, 1000, 2000];
    
    for file_count in file_counts {
        println!("Testing directory with {} files", file_count);
        
        // Clean directory
        for entry in fs::read_dir(temp_dir.path()).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        
        // Create files
        let creation_start = Instant::now();
        for i in 0..file_count {
            let filename = format!("perf_test_{:06}.txt", i);
            let content = format!("Performance test file {} with some content", i);
            let file_path = temp_dir.path().join(&filename);
            fs::write(&file_path, &content).unwrap();
        }
        let creation_time = creation_start.elapsed();
        
        // List directory
        let listing_start = Instant::now();
        let files = list_directory(temp_dir.path()).unwrap();
        let listing_time = listing_start.elapsed();
        
        // Verify
        assert_eq!(files.len(), file_count);
        
        // Calculate performance metrics
        let creation_rate = file_count as f64 / creation_time.as_secs_f64();
        let listing_rate = file_count as f64 / listing_time.as_secs_f64();
        
        println!("  - Creation: {:?} ({:.2} files/sec)", creation_time, creation_rate);
        println!("  - Listing: {:?} ({:.2} files/sec)", listing_time, listing_rate);
        
        // Performance assertions (these may need adjustment based on hardware)
        assert!(creation_time < Duration::from_secs(30), "File creation too slow");
        assert!(listing_time < Duration::from_secs(5), "Directory listing too slow");
    }
}

#[test]
fn test_large_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test various file sizes
    let sizes = vec![
        (1024, "1KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
        (50 * 1024 * 1024, "50MB"),
    ];
    
    for (size, size_name) in sizes {
        println!("Testing {} file", size_name);
        
        let filename = format!("large_file_{}.bin", size_name);
        let file_path = temp_dir.path().join(&filename);
        
        // Create large file
        let creation_start = Instant::now();
        let content = vec![0x42u8; size];
        fs::write(&file_path, &content).unwrap();
        let creation_time = creation_start.elapsed();
        
        // Get file info
        let info_start = Instant::now();
        let file_info = get_file_info(&file_path).unwrap();
        let info_time = info_start.elapsed();
        
        // Verify
        assert_eq!(file_info.size, size as u64);
        assert_eq!(file_info.name, filename);
        
        println!("  - Creation: {:?}", creation_time);
        println!("  - Info retrieval: {:?}", info_time);
        
        // Performance assertions
        assert!(creation_time < Duration::from_secs(60), "Large file creation too slow");
        assert!(info_time < Duration::from_secs(1), "File info retrieval too slow");
        
        // Clean up
        fs::remove_file(&file_path).unwrap();
    }
}

#[test]
fn test_device_info_generation_performance() {
    let iterations = 1000;
    
    println!("Testing DeviceInfo generation performance with {} iterations", iterations);
    
    let start_time = Instant::now();
    let mut device_infos = Vec::with_capacity(iterations);
    
    for _ in 0..iterations {
        let device_info = DeviceInfo::new(8080);
        device_infos.push(device_info);
    }
    
    let generation_time = start_time.elapsed();
    
    // Verify all devices have unique IDs
    let mut ids = std::collections::HashSet::new();
    for device in &device_infos {
        assert!(ids.insert(device.id.clone()), "Duplicate device ID found");
    }
    
    let rate = iterations as f64 / generation_time.as_secs_f64();
    
    println!("  - Total time: {:?}", generation_time);
    println!("  - Rate: {:.2} devices/sec", rate);
    
    // Performance assertions
    assert!(generation_time < Duration::from_secs(10), "DeviceInfo generation too slow");
    assert!(rate > 100.0, "DeviceInfo generation rate too low");
}

#[test]
fn test_uuid_generation_consistency_under_load() {
    let temp_dir = TempDir::new().unwrap();
    let num_threads = 8;
    let iterations_per_thread = 100;
    
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];
    
    // Create a test file
    let test_file = temp_dir.path().join("uuid_test.txt");
    fs::write(&test_file, "UUID consistency test").unwrap();
    
    for _thread_id in 0..num_threads {
        let barrier = Arc::clone(&barrier);
        let test_file = test_file.clone();
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            let mut uuids = Vec::with_capacity(iterations_per_thread);
            
            for _ in 0..iterations_per_thread {
                let file_info = get_file_info(&test_file).unwrap();
                uuids.push(file_info.id);
            }
            
            uuids
        });
        
        handles.push(handle);
    }
    
    // Collect all UUIDs
    let mut all_uuids = Vec::new();
    for handle in handles {
        let thread_uuids = handle.join().unwrap();
        all_uuids.extend(thread_uuids);
    }
    
    // All UUIDs should be identical (deterministic)
    let first_uuid = all_uuids[0];
    for uuid in &all_uuids {
        assert_eq!(*uuid, first_uuid, "UUID generation not deterministic under load");
    }
    
    println!("UUID consistency test passed with {} total operations", all_uuids.len());
}

#[test]
fn test_port_availability_under_stress() {
    let num_threads = 10;
    let checks_per_thread = 100;
    
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = vec![];
    
    for _thread_id in 0..num_threads {
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            let start_port = 50000 + (_thread_id as u16 * 100);
            let end_port = start_port + 99;
            
            for _ in 0..checks_per_thread {
                // Test port availability
                let is_available = is_port_available(start_port);
                
                // Find available port in range
                let found_port = find_available_port(start_port, end_port);
                
                // At least one port should be available in the range
                assert!(found_port.is_some() || !is_available, 
                    "No ports available in range {}-{}", start_port, end_port);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("Port availability stress test completed successfully");
}

#[test]
fn test_memory_usage_with_many_files() {
    let temp_dir = TempDir::new().unwrap();
    let file_count = 5000;
    
    println!("Testing memory usage with {} files", file_count);
    
    // Create many small files
    for i in 0..file_count {
        let filename = format!("memory_test_{:06}.txt", i);
        let content = format!("Memory test file {}", i);
        let file_path = temp_dir.path().join(&filename);
        fs::write(&file_path, &content).unwrap();
    }
    
    // List directory multiple times to test memory stability
    for iteration in 0..10 {
        let start_time = Instant::now();
        let files = list_directory(temp_dir.path()).unwrap();
        let elapsed = start_time.elapsed();
        
        assert_eq!(files.len(), file_count);
        
        println!("  Iteration {}: {:?}", iteration + 1, elapsed);
        
        // Each iteration should be reasonably fast
        assert!(elapsed < Duration::from_secs(5), 
            "Directory listing too slow on iteration {}", iteration + 1);
    }
    
    println!("Memory usage test completed successfully");
}

#[test]
fn test_file_name_edge_cases_stress() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test many files with challenging names
    let problematic_names = vec![
        "file with spaces.txt",
        "file-with-dashes.txt", 
        "file_with_underscores.txt",
        "file.with.many.dots.txt",
        "UPPERCASE.TXT",
        "lowercase.txt",
        "MiXeD-CaSe_File.TXT",
        "123-numeric-start.txt",
        "file(with)parentheses.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
        "file'with'quotes.txt",
        "file\"with\"doublequotes.txt",
        "file@with@symbols.txt",
        "file#with#hash.txt",
        "file$with$dollar.txt",
        "file%with%percent.txt",
        "file&with&ampersand.txt",
        "file+with+plus.txt",
        "file=with=equals.txt",
        "file~with~tilde.txt",
        "file`with`backtick.txt",
    ];
    
    // Create many files with each problematic name pattern
    let mut created_files = 0;
    
    for (i, base_name) in problematic_names.iter().enumerate() {
        for j in 0..50 { // Create 50 variants of each
            let filename = format!("{}_{:03}", base_name, j);
            let file_path = temp_dir.path().join(&filename);
            
            // Try to create the file (some names might not be valid on all filesystems)
            if let Ok(_) = fs::write(&file_path, format!("Content {}-{}", i, j)) {
                created_files += 1;
            }
        }
    }
    
    println!("Created {} files with problematic names", created_files);
    
    // List directory should handle all these files
    let start_time = Instant::now();
    let files = list_directory(temp_dir.path()).unwrap();
    let listing_time = start_time.elapsed();
    
    assert_eq!(files.len(), created_files);
    
    println!("Listed {} files in {:?}", files.len(), listing_time);
    
    // Should complete in reasonable time
    assert!(listing_time < Duration::from_secs(10), "Listing took too long with problematic filenames");
    
    // Verify all files have valid info
    for file in &files {
        assert!(!file.name.is_empty(), "File name should not be empty");
        assert!(file.size > 0, "File should have content");
        assert!(!file.size_human.is_empty(), "Human readable size should not be empty");
    }
} 