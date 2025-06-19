use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustdrop::utils::file::{get_file_info, list_directory};
use rustdrop::core::config::AppConfig;
use rustdrop::core::models::DeviceInfo;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

// Benchmark file operations
fn bench_file_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files of various sizes
    let test_files = vec![
        ("small.txt", 1024),      // 1KB
        ("medium.txt", 1024 * 1024), // 1MB
        ("large.txt", 10 * 1024 * 1024), // 10MB
    ];
    
    for (name, size) in &test_files {
        let file_path = temp_dir.path().join(name);
        let mut file = File::create(&file_path).unwrap();
        let data = vec![b'A'; *size];
        file.write_all(&data).unwrap();
    }
    
    let mut group = c.benchmark_group("file_operations");
    
    // Benchmark get_file_info for different file sizes
    for (name, _) in &test_files {
        let file_path = temp_dir.path().join(name);
        group.bench_with_input(
            BenchmarkId::new("get_file_info", name),
            &file_path,
            |b, path| {
                b.iter(|| get_file_info(black_box(path)))
            }
        );
    }
    
    // Benchmark directory listing
    group.bench_function("list_directory", |b| {
        b.iter(|| list_directory(black_box(temp_dir.path())))
    });
    
    group.finish();
}

// Benchmark configuration operations
fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");
    
    // Benchmark default config creation
    group.bench_function("config_default", |b| {
        b.iter(|| AppConfig::default())
    });
    
    // Benchmark TOML parsing
    let toml_content = r#"
        [server]
        port = 8080
        host = "0.0.0.0"
        max_file_size = 1073741824
        
        [files]
        expiry_hours = 24
        
        [discovery]
        enabled = true
        
        [ui]
        qr_code = true
        open_browser = false
    "#;
    
    group.bench_function("config_from_toml", |b| {
        b.iter(|| AppConfig::from_toml(black_box(toml_content)))
    });
    
    group.finish();
}

// Benchmark device info operations
fn bench_device_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("device_operations");
    
    // Benchmark device info creation
    group.bench_function("device_info_new", |b| {
        b.iter(|| DeviceInfo::new(black_box(8080)))
    });
    
    // Benchmark URL generation
    let device_info = DeviceInfo::new(8080);
    group.bench_function("device_info_url", |b| {
        b.iter(|| device_info.url())
    });
    
    group.finish();
}

// Benchmark large directory listing
fn bench_large_directory(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    
    // Create many files
    let file_counts = vec![10, 100, 1000];
    
    for &count in &file_counts {
        let test_dir = temp_dir.path().join(format!("test_{}", count));
        std::fs::create_dir(&test_dir).unwrap();
        
        for i in 0..count {
            let file_path = test_dir.join(format!("file_{:04}.txt", i));
            std::fs::write(&file_path, format!("Content {}", i)).unwrap();
        }
        
        c.bench_with_input(
            BenchmarkId::new("large_directory_listing", count),
            &test_dir,
            |b, dir| {
                b.iter(|| list_directory(black_box(dir)))
            }
        );
    }
}

// Benchmark UUID generation consistency
fn bench_uuid_generation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("uuid_test.txt");
    std::fs::write(&file_path, "test content").unwrap();
    
    let mut group = c.benchmark_group("uuid_generation");
    
    // Benchmark UUID generation for the same file (should be deterministic)
    group.bench_function("deterministic_uuid", |b| {
        b.iter(|| {
            let info = get_file_info(black_box(&file_path)).unwrap();
            black_box(info.id)
        })
    });
    
    group.finish();
}

// Benchmark concurrent file operations
fn bench_concurrent_operations(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..50 {
        let file_path = temp_dir.path().join(format!("concurrent_{}.txt", i));
        std::fs::write(&file_path, format!("Content {}", i)).unwrap();
    }
    
    let temp_path = Arc::new(temp_dir.path().to_path_buf());
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    group.bench_function("concurrent_file_info", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..10).map(|i| {
                let path = temp_path.clone();
                thread::spawn(move || {
                    let file_path = path.join(format!("concurrent_{}.txt", i));
                    get_file_info(&file_path)
                })
            }).collect();
            
            for handle in handles {
                black_box(handle.join().unwrap());
            }
        })
    });
    
    group.finish();
}

// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with different sizes to test memory usage
    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024]; // 1KB to 1MB
    
    for &size in &sizes {
        let file_path = temp_dir.path().join(format!("memory_test_{}.bin", size));
        let data = vec![0u8; size];
        std::fs::write(&file_path, &data).unwrap();
        
        c.bench_with_input(
            BenchmarkId::new("memory_file_info", size),
            &file_path,
            |b, path| {
                b.iter(|| {
                    let info = get_file_info(black_box(path)).unwrap();
                    black_box(info)
                })
            }
        );
    }
}

criterion_group!(
    benches,
    bench_file_operations,
    bench_config_operations,
    bench_device_operations,
    bench_large_directory,
    bench_uuid_generation,
    bench_concurrent_operations,
    bench_memory_patterns
);

criterion_main!(benches); 