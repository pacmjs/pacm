use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pacm_resolver::{resolve_full_tree_async, ResolvedPackage};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

fn resolve_simple_dependency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = Arc::new(reqwest::Client::new());

    c.bench_function("resolve_lodash", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut seen = HashSet::new();
                let _ =
                    resolve_full_tree_async(client.clone(), "lodash", "^4.17.21", &mut seen).await;
            });
        })
    });
}

fn resolve_complex_dependency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = Arc::new(reqwest::Client::new());

    let mut group = c.benchmark_group("resolve_complex");
    group.measurement_time(Duration::from_secs(30));

    let complex_packages = vec![
        ("express", "^4.18.0"),
        ("typescript", "^4.9.0"),
        ("webpack", "^5.75.0"),
        ("@babel/core", "^7.20.0"),
    ];

    for (package, version) in complex_packages {
        group.bench_with_input(
            BenchmarkId::new("package", package),
            &(package, version),
            |b, &(pkg_name, pkg_version)| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut seen = HashSet::new();
                        let _ = resolve_full_tree_async(
                            client.clone(),
                            pkg_name,
                            pkg_version,
                            &mut seen,
                        )
                        .await;
                    });
                });
            },
        );
    }
    group.finish();
}

fn resolve_multiple_dependencies(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = Arc::new(reqwest::Client::new());

    let mut group = c.benchmark_group("resolve_multiple");
    group.measurement_time(Duration::from_secs(60));

    let dependency_sets = vec![
        vec![("lodash", "^4.17.21")],
        vec![("lodash", "^4.17.21"), ("chalk", "^4.1.2")],
        vec![
            ("lodash", "^4.17.21"),
            ("chalk", "^4.1.2"),
            ("debug", "^4.3.2"),
        ],
        vec![
            ("express", "^4.18.0"),
            ("cors", "^2.8.5"),
            ("helmet", "^6.0.0"),
        ],
    ];

    for (i, deps) in dependency_sets.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("deps", i + 1), deps, |b, deps| {
            b.iter(|| {
                rt.block_on(async {
                    let mut all_resolved = Vec::new();
                    for (pkg_name, pkg_version) in deps {
                        let mut seen = HashSet::new();
                        if let Ok(resolved) = resolve_full_tree_async(
                            client.clone(),
                            pkg_name,
                            pkg_version,
                            &mut seen,
                        )
                        .await
                        {
                            all_resolved.extend(resolved);
                        }
                    }
                });
            });
        });
    }
    group.finish();
}

fn resolve_scoped_packages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = Arc::new(reqwest::Client::new());

    let mut group = c.benchmark_group("resolve_scoped");

    let scoped_packages = vec![
        ("@types/node", "^18.11.0"),
        ("@babel/core", "^7.20.0"),
        ("@vue/cli", "^5.0.0"),
    ];

    for (package, version) in scoped_packages {
        group.bench_with_input(
            BenchmarkId::new("scoped", package.replace('/', "_").replace('@', "at_")),
            &(package, version),
            |b, &(pkg_name, pkg_version)| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut seen = HashSet::new();
                        let _ = resolve_full_tree_async(
                            client.clone(),
                            pkg_name,
                            pkg_version,
                            &mut seen,
                        )
                        .await;
                    });
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    resolve_simple_dependency,
    resolve_complex_dependency,
    resolve_multiple_dependencies,
    resolve_scoped_packages
);
criterion_main!(benches);
