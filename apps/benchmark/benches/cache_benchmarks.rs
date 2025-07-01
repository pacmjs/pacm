use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pacm_store::{get_store_path, StoreManager};
use std::time::Duration;

fn cache_lookup_operations(c: &mut Criterion) {
    let store_manager = StoreManager::new();
    let store_path = get_store_path();

    let mut group = c.benchmark_group("cache_lookup");

    let test_packages = vec![
        ("lodash", "4.17.21"),
        ("@types/node", "18.11.0"),
        ("express", "4.18.0"),
        ("nonexistent-package", "1.0.0"),
    ];

    for (package, version) in test_packages {
        group.bench_with_input(
            BenchmarkId::new("lookup", format!("{}@{}", package, version)),
            &(package, version),
            |b, &(pkg_name, pkg_version)| {
                b.iter(|| {
                    let package_key = format!("{}@{}", pkg_name, pkg_version);
                    let safe_name = if pkg_name.starts_with('@') {
                        pkg_name.replace('@', "_at_").replace('/', "_slash_")
                    } else {
                        pkg_name.to_string()
                    };

                    let npm_dir = store_path.join("npm");
                    if npm_dir.exists() {
                        let package_prefix = format!("{safe_name}@{}-", pkg_version);
                        if let Ok(entries) = std::fs::read_dir(&npm_dir) {
                            for entry in entries.flatten() {
                                let dir_name = entry.file_name();
                                if let Some(name_str) = dir_name.to_str() {
                                    if name_str.starts_with(&package_prefix) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

fn cache_store_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_store");
    group.measurement_time(Duration::from_secs(30));

    let test_data_sizes = vec![
        ("small", 1024),    // 1KB
        ("medium", 102400), // 100KB
        ("large", 1048576), // 1MB
    ];

    for (size_name, data_size) in test_data_sizes {
        group.bench_with_input(
            BenchmarkId::new("store", size_name),
            &data_size,
            |b, &size| {
                b.iter(|| {
                    let test_data = vec![0u8; size];
                    let temp_file =
                        format!("test_cache_{}_{}.tmp", size_name, uuid::Uuid::new_v4());
                    let temp_path = std::env::temp_dir().join(temp_file);

                    let _ = std::fs::write(&temp_path, &test_data);

                    let _ = std::fs::remove_file(&temp_path);
                });
            },
        );
    }
    group.finish();
}

fn cache_index_building(c: &mut Criterion) {
    c.bench_function("build_cache_index", |b| {
        b.iter(|| {
            let store_path = get_store_path();
            let npm_dir = store_path.join("npm");

            if npm_dir.exists() {
                let mut cache_index = std::collections::HashMap::new();

                if let Ok(entries) = std::fs::read_dir(&npm_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let dir_name = entry.file_name();
                            if let Some(name_str) = dir_name.to_str() {
                                if let Some(at_pos) = name_str.rfind('@') {
                                    if let Some(dash_pos) = name_str[at_pos..].find('-') {
                                        let package_name = &name_str[..at_pos];
                                        let version_part = &name_str[at_pos + 1..at_pos + dash_pos];
                                        cache_index.insert(
                                            format!("{}@{}", package_name, version_part),
                                            entry.path(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    });
}

fn cache_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_batch");

    let batch_sizes = vec![10, 50, 100, 500];

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("batch_lookup", batch_size),
            &batch_size,
            |b, &size| {
                let packages: Vec<String> = (0..size)
                    .map(|i| format!("test-package-{}@1.0.{}", i, i % 10))
                    .collect();

                b.iter(|| {
                    let store_path = get_store_path();
                    let npm_dir = store_path.join("npm");
                    let mut found_packages = Vec::new();

                    if npm_dir.exists() {
                        for package_key in &packages {
                            if let Some(at_pos) = package_key.find('@') {
                                let package_name = &package_key[..at_pos];
                                let version = &package_key[at_pos + 1..];

                                let safe_name = if package_name.starts_with('@') {
                                    package_name.replace('@', "_at_").replace('/', "_slash_")
                                } else {
                                    package_name.to_string()
                                };

                                let package_prefix = format!("{safe_name}@{}-", version);

                                if let Ok(entries) = std::fs::read_dir(&npm_dir) {
                                    for entry in entries.flatten() {
                                        let dir_name = entry.file_name();
                                        if let Some(name_str) = dir_name.to_str() {
                                            if name_str.starts_with(&package_prefix) {
                                                found_packages.push(entry.path());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    cache_lookup_operations,
    cache_store_operations,
    cache_index_building,
    cache_batch_operations
);
criterion_main!(benches);
