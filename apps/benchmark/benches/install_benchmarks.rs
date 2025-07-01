use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pacm_core::InstallManager;
use pacm_project::DependencyType;
use std::time::Duration;
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

fn install_small_package(c: &mut Criterion) {
    c.bench_function("install_lodash", |b| {
        b.iter(|| {
            let temp_dir = create_temp_project();
            let manager = InstallManager::new();

            let _ = manager.install_single_dependency(
                temp_dir.path().to_str().unwrap(),
                "lodash",
                "latest",
                DependencyType::Dependency,
                false,
                true, // no save
                false,
                false,
            );
        })
    });
}

fn install_medium_package(c: &mut Criterion) {
    c.bench_function("install_express", |b| {
        b.iter(|| {
            let temp_dir = create_temp_project();
            let manager = InstallManager::new();

            let _ = manager.install_single_dependency(
                temp_dir.path().to_str().unwrap(),
                "express",
                "latest",
                DependencyType::Dependency,
                false,
                true,
                false,
                false,
            );
        })
    });
}

fn install_multiple_packages(c: &mut Criterion) {
    let packages = vec!["lodash", "chalk", "debug"];

    let mut group = c.benchmark_group("install_multiple");
    group.measurement_time(Duration::from_secs(60));

    for package_count in [1, 3, 5].iter() {
        group.bench_with_input(
            BenchmarkId::new("packages", package_count),
            package_count,
            |b, &package_count| {
                b.iter(|| {
                    let temp_dir = create_temp_project();
                    let manager = InstallManager::new();

                    for i in 0..(package_count.min(packages.len())) {
                        let _ = manager.install_single_dependency(
                            temp_dir.path().to_str().unwrap(),
                            packages[i],
                            "latest",
                            DependencyType::Dependency,
                            false,
                            true,
                            false,
                            false,
                        );
                    }
                });
            },
        );
    }
    group.finish();
}

fn install_all_dependencies(c: &mut Criterion) {
    c.bench_function("install_all", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let package_json_content = r#"{
  "name": "benchmark-test-project",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.21",
    "chalk": "^4.1.2",
    "debug": "^4.3.2"
  }
}"#;
            std::fs::write(temp_dir.path().join("package.json"), package_json_content).unwrap();

            let manager = InstallManager::new();
            let _ = manager.install_all(temp_dir.path().to_str().unwrap(), false);
        })
    });
}

criterion_group!(
    benches,
    install_small_package,
    install_medium_package,
    install_multiple_packages,
    install_all_dependencies
);
criterion_main!(benches);
