use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pacm_core::InstallManager;
use pacm_project::DependencyType;
use std::time::Duration;
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use tempfile::TempDir;

fn create_temp_project() -> TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    let package_json_content = r#"{
  "name": "benchmark-test-project",
  "version": "1.0.0",
  "description": "Temporary project for benchmarking",
  "main": "index.js",
  "dependencies": {},
  "devDependencies": {}
}"#;
    std::fs::write(temp_dir.path().join("package.json"), package_json_content).unwrap();
    temp_dir
}

fn memory_usage_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut system = System::new_all();

    c.bench_function("memory_usage_small_install", |b| {
        b.iter(|| {
            system.refresh_all();
            let initial_memory = get_current_memory_usage(&system);

            let temp_dir = create_temp_project();
            let manager = InstallManager::new();

            rt.block_on(async {
                let _ = manager.install_single_dependency(
                    temp_dir.path().to_str().unwrap(),
                    "lodash",
                    "4.17.21",
                    DependencyType::Dependency,
                    false,
                    true, // no save
                    false,
                    false,
                );
            });

            system.refresh_all();
            let final_memory = get_current_memory_usage(&system);
            let memory_delta = final_memory - initial_memory;

            println!("Memory delta: {} MB", memory_delta / 1024 / 1024);
        })
    });
}

fn concurrent_operations_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("concurrent_small_installs", |b| {
        b.iter(|| {
            let packages = vec![
                ("lodash", "4.17.21"),
                ("chalk", "^4.1.2"),
                ("debug", "^4.3.2"),
            ];

            rt.block_on(async {
                let futures: Vec<_> = packages
                    .into_iter()
                    .map(|(name, version)| {
                        let temp_dir = create_temp_project();
                        let manager = InstallManager::new();

                        async move {
                            manager
                                .install_single_dependency(
                                    temp_dir.path().to_str().unwrap(),
                                    name,
                                    version,
                                    DependencyType::Dependency,
                                    false,
                                    true, // no save
                                    false,
                                    false,
                                )
                                .await
                        }
                    })
                    .collect();

                futures::future::join_all(futures).await;
            });
        })
    });
}

fn stress_test_many_packages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let package_counts = vec![5, 10, 20];

    for count in package_counts {
        c.bench_with_input(
            BenchmarkId::new("stress_test_packages", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let packages = generate_test_packages(count);
                    let temp_dir = create_temp_project();
                    let manager = InstallManager::new();

                    rt.block_on(async {
                        for (name, version) in packages {
                            let _ = manager
                                .install_single_dependency(
                                    temp_dir.path().to_str().unwrap(),
                                    name,
                                    version,
                                    DependencyType::Dependency,
                                    false,
                                    true, // no save
                                    false,
                                    false,
                                )
                                .await;
                        }
                    });
                })
            },
        );
    }
}

fn cache_performance_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("cache_hit_performance", |b| {
        let temp_dir = create_temp_project();
        let manager = InstallManager::new();

        rt.block_on(async {
            let _ = manager
                .install_single_dependency(
                    temp_dir.path().to_str().unwrap(),
                    "lodash",
                    "4.17.21",
                    DependencyType::Dependency,
                    false,
                    true,
                    false,
                    false,
                )
                .await;
        });

        b.iter(|| {
            let temp_dir2 = create_temp_project();

            rt.block_on(async {
                let _ = manager
                    .install_single_dependency(
                        temp_dir2.path().to_str().unwrap(),
                        "lodash",
                        "4.17.21",
                        DependencyType::Dependency,
                        false,
                        true,
                        false,
                        false,
                    )
                    .await;
            });
        })
    });
}

fn get_current_memory_usage(system: &System) -> u64 {
    if let Some(process) = system.process(sysinfo::get_current_pid().ok().unwrap()) {
        process.memory()
    } else {
        0
    }
}

fn generate_test_packages(count: usize) -> Vec<(&'static str, &'static str)> {
    let all_packages = vec![
        ("lodash", "4.17.21"),
        ("chalk", "^4.1.2"),
        ("debug", "^4.3.2"),
        ("ms", "^2.1.2"),
        ("is-number", "^7.0.0"),
        ("mime", "^3.0.0"),
        ("qs", "^6.11.0"),
        ("uuid", "^9.0.0"),
        ("validator", "^13.7.0"),
        ("semver", "^7.3.8"),
        ("moment", "^2.29.0"),
        ("axios", "^1.2.0"),
        ("express", "^4.18.0"),
        ("react", "^18.2.0"),
        ("vue", "^3.2.0"),
    ];

    all_packages.into_iter().take(count).collect()
}

criterion_group!(
    system_benches,
    memory_usage_benchmark,
    concurrent_operations_benchmark,
    stress_test_many_packages,
    cache_performance_benchmark
);
criterion_main!(system_benches);
