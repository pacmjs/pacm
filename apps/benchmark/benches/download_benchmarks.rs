use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pacm_core::download::PackageDownloader;
use pacm_resolver::ResolvedPackage;
use std::collections::HashMap;
use std::time::Duration;

fn download_small_packages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("download_small");
    group.measurement_time(Duration::from_secs(30));

    let small_packages = vec![
        (
            "lodash",
            "4.17.21",
            "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
        ),
        (
            "chalk",
            "4.1.2",
            "https://registry.npmjs.org/chalk/-/chalk-4.1.2.tgz",
        ),
        (
            "debug",
            "4.3.4",
            "https://registry.npmjs.org/debug/-/debug-4.3.4.tgz",
        ),
    ];

    for (package, version, tarball_url) in small_packages {
        group.bench_with_input(
            BenchmarkId::new("package", package),
            &(package, version, tarball_url),
            |b, &(pkg_name, pkg_version, pkg_url)| {
                b.iter(|| {
                    rt.block_on(async {
                        let downloader = PackageDownloader::new();
                        let resolved_package = ResolvedPackage {
                            name: pkg_name.to_string(),
                            version: pkg_version.to_string(),
                            resolved: pkg_url.to_string(),
                            integrity: "sha512-mock-integrity".to_string(),
                            dependencies: HashMap::new(),
                        };

                        let _ = downloader.download_single(&resolved_package, false).await;
                    });
                });
            },
        );
    }
    group.finish();
}

fn download_parallel_packages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("download_parallel");
    group.measurement_time(Duration::from_secs(60));

    let package_batches = vec![
        vec![(
            "lodash",
            "4.17.21",
            "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
        )],
        vec![
            (
                "lodash",
                "4.17.21",
                "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
            ),
            (
                "chalk",
                "4.1.2",
                "https://registry.npmjs.org/chalk/-/chalk-4.1.2.tgz",
            ),
        ],
        vec![
            (
                "lodash",
                "4.17.21",
                "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
            ),
            (
                "chalk",
                "4.1.2",
                "https://registry.npmjs.org/chalk/-/chalk-4.1.2.tgz",
            ),
            (
                "debug",
                "4.3.4",
                "https://registry.npmjs.org/debug/-/debug-4.3.4.tgz",
            ),
        ],
    ];

    for (i, batch) in package_batches.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("batch", i + 1), batch, |b, batch| {
            b.iter(|| {
                rt.block_on(async {
                    let downloader = PackageDownloader::new();
                    let resolved_packages: Vec<ResolvedPackage> = batch
                        .iter()
                        .map(|(name, version, url)| ResolvedPackage {
                            name: name.to_string(),
                            version: version.to_string(),
                            resolved: url.to_string(),
                            integrity: "sha512-mock-integrity".to_string(),
                            dependencies: HashMap::new(),
                        })
                        .collect();

                    let _ = downloader
                        .download_parallel(&resolved_packages, false)
                        .await;
                });
            });
        });
    }
    group.finish();
}

fn download_with_concurrency_limits(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("download_concurrency");
    group.measurement_time(Duration::from_secs(45));

    let concurrency_levels = vec![1, 5, 10, 20];
    let test_packages: Vec<ResolvedPackage> = vec![
        "lodash",
        "chalk",
        "debug",
        "ms",
        "semver",
        "minimist",
        "glob",
        "rimraf",
        "mkdirp",
        "commander",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| ResolvedPackage {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        resolved: format!("https://registry.npmjs.org/{}/-/{}-1.0.0.tgz", name, name),
        integrity: "sha512-mock-integrity".to_string(),
        dependencies: HashMap::new(),
    })
    .collect();

    for concurrency in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            &concurrency,
            |b, &_concurrency| {
                b.iter(|| {
                    rt.block_on(async {
                        let downloader = PackageDownloader::new();
                        let _ = downloader.download_parallel(&test_packages, false).await;
                    });
                });
            },
        );
    }
    group.finish();
}

fn download_retry_mechanisms(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("download_with_retries", |b| {
        b.iter(|| {
            rt.block_on(async {
                let downloader = PackageDownloader::new();
                let _ = downloader.download_single(&failing_package, false).await;
                let failing_package = ResolvedPackage {
                    name: "nonexistent-test-package".to_string(),
                    version: "1.0.0".to_string(),
                    resolved: "https://registry.npmjs.org/nonexistent-test-package/-/nonexistent-test-package-1.0.0.tgz".to_string(),
                    integrity: "sha512-mock-integrity".to_string(),
                    dependencies: HashMap::new(),
                };
                let _ = downloader.download_single(&failing_package, false).await;
            });
        });
    });
}

fn download_different_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("download_sizes");
    group.measurement_time(Duration::from_secs(60));

    let size_categories = vec![
        ("small", "lodash", "4.17.21"),   // ~500KB
        ("medium", "express", "4.18.2"),  // ~2MB with deps
        ("large", "typescript", "4.9.5"), // ~60MB
    ];

    for (size_category, package, version) in size_categories {
        group.bench_with_input(
            BenchmarkId::new("size", size_category),
            &(package, version),
            |b, &(pkg_name, pkg_version)| {
                b.iter(|| {
                    rt.block_on(async {
                        let downloader = PackageDownloader::new();
                        let resolved_package = ResolvedPackage {
                            name: pkg_name.to_string(),
                            version: pkg_version.to_string(),
                            resolved: format!(
                                "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                pkg_name, pkg_name, pkg_version
                            ),
                            integrity: "sha512-mock-integrity".to_string(),
                            dependencies: HashMap::new(),
                        };

                        let _ = downloader.download_single(&resolved_package, false).await;
                    });
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    download_small_packages,
    download_parallel_packages,
    download_with_concurrency_limits,
    download_retry_mechanisms,
    download_different_sizes
);
criterion_main!(benches);
