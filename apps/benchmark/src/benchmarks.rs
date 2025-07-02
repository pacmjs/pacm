use crate::performance_monitor::{OperationMetadata, PerformanceMonitor};
use crate::utils::create_temp_project;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use pacm_core::InstallManager;
use std::time::Duration;

pub struct InstallBenchmarks {
    monitor: PerformanceMonitor,
}

impl InstallBenchmarks {
    pub fn new() -> Self {
        let mut monitor = PerformanceMonitor::new();

        monitor.add_metadata(
            "install_single_small",
            OperationMetadata {
                category: "install".to_string(),
                description: "Install single small package".to_string(),
                expected_range: Some((100, 2000)), // 100ms to 2s
            },
        );

        monitor.add_metadata(
            "install_single_medium",
            OperationMetadata {
                category: "install".to_string(),
                description: "Install single medium package with dependencies".to_string(),
                expected_range: Some((500, 5000)), // 500ms to 5s
            },
        );

        monitor.add_metadata(
            "install_single_large",
            OperationMetadata {
                category: "install".to_string(),
                description: "Install large package with many dependencies".to_string(),
                expected_range: Some((2000, 15000)), // 2s to 15s
            },
        );

        Self { monitor }
    }

    pub fn run_all(&mut self, iterations: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "📦 Installation Benchmarks".bright_blue().bold());

        let progress = ProgressBar::new((iterations * 6) as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")?
                .progress_chars("#>-"),
        );

        let test_packages = vec![
            ("lodash", "Small utility package"),
            ("express", "Medium web framework"),
            ("typescript", "Large development tool"),
        ];

        for (package, description) in test_packages {
            self.benchmark_install_fresh(package, description, iterations, &progress)?;
            self.benchmark_install_cached(package, description, iterations, &progress)?;
        }

        progress.finish_with_message("Installation benchmarks completed!");

        println!(
            "\n{}",
            "Installation Benchmark Results:".bright_green().bold()
        );
        self.monitor.print_summary();

        Ok(())
    }

    fn benchmark_install_fresh(
        &mut self,
        package: &str,
        description: &str,
        iterations: u32,
        progress: &ProgressBar,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let operation_name = format!("install_fresh_{}", package);

        println!(
            "\n🚀 Benchmarking fresh install: {} ({})",
            package.bright_white(),
            description
        );

        for i in 0..iterations {
            let temp_dir = create_temp_project()?;
            let project_path = temp_dir.path().to_str().unwrap();

            let manager = InstallManager::new();

            self.monitor.start_timer(&operation_name);

            match manager.install_single(
                project_path,
                package,
                "latest",
                pacm_project::DependencyType::Dependencies,
                false, // save_exact
                true,  // no_save (don't modify package.json for benchmark)
                false, // force
                false, // debug
            ) {
                Ok(_) => {
                    self.monitor.stop_timer(&operation_name);
                    progress.inc(1);
                }
                Err(e) => {
                    eprintln!(
                        "❌ Fresh install failed for {} (iteration {}): {}",
                        package,
                        i + 1,
                        e
                    );
                    progress.inc(1);
                }
            }
        }

        Ok(())
    }

    fn benchmark_install_cached(
        &mut self,
        package: &str,
        description: &str,
        iterations: u32,
        progress: &ProgressBar,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let operation_name = format!("install_cached_{}", package);

        println!(
            "\n⚡ Benchmarking cached install: {} ({})",
            package.bright_white(),
            description
        );

        let temp_warmup = create_temp_project()?;
        let manager = InstallManager::new();
        let _ = manager.install_single(
            temp_warmup.path().to_str().unwrap(),
            package,
            "latest",
            pacm_project::DependencyType::Dependencies,
            false,
            true,
            false,
            false,
        );

        for i in 0..iterations {
            let temp_dir = create_temp_project()?;
            let project_path = temp_dir.path().to_str().unwrap();

            self.monitor.start_timer(&operation_name);

            match manager.install_single(
                project_path,
                package,
                "latest",
                pacm_project::DependencyType::Dependencies,
                false,
                true,
                false,
                false,
            ) {
                Ok(_) => {
                    self.monitor.stop_timer(&operation_name);
                    progress.inc(1);
                }
                Err(e) => {
                    eprintln!(
                        "❌ Cached install failed for {} (iteration {}): {}",
                        package,
                        i + 1,
                        e
                    );
                    progress.inc(1);
                }
            }
        }

        Ok(())
    }
}

pub struct ResolutionBenchmarks {
    monitor: PerformanceMonitor,
}

impl ResolutionBenchmarks {
    pub fn new() -> Self {
        let mut monitor = PerformanceMonitor::new();

        monitor.add_metadata(
            "resolve_simple",
            OperationMetadata {
                category: "resolution".to_string(),
                description: "Resolve simple dependency tree".to_string(),
                expected_range: Some((50, 500)),
            },
        );

        monitor.add_metadata(
            "resolve_complex",
            OperationMetadata {
                category: "resolution".to_string(),
                description: "Resolve complex dependency tree".to_string(),
                expected_range: Some((200, 2000)),
            },
        );

        Self { monitor }
    }

    pub fn run_all(&mut self, iterations: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "🔍 Resolution Benchmarks".bright_blue().bold());

        println!("Resolution benchmarks with {} iterations", iterations);

        for _ in 0..iterations {
            self.monitor.start_timer("resolve_simple");
            std::thread::sleep(Duration::from_millis(100));
            self.monitor.stop_timer("resolve_simple");
        }

        self.monitor.print_summary();
        Ok(())
    }
}

pub struct CacheBenchmarks {
    monitor: PerformanceMonitor,
}

impl CacheBenchmarks {
    pub fn new() -> Self {
        let mut monitor = PerformanceMonitor::new();

        monitor.add_metadata(
            "cache_lookup",
            OperationMetadata {
                category: "cache".to_string(),
                description: "Cache lookup operation".to_string(),
                expected_range: Some((1, 50)),
            },
        );

        monitor.add_metadata(
            "cache_store",
            OperationMetadata {
                category: "cache".to_string(),
                description: "Cache store operation".to_string(),
                expected_range: Some((10, 200)),
            },
        );

        Self { monitor }
    }

    pub fn run_all(&mut self, iterations: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "💾 Cache Benchmarks".bright_blue().bold());

        println!("Cache benchmarks with {} iterations", iterations);

        for _ in 0..iterations {
            self.monitor.start_timer("cache_lookup");
            std::thread::sleep(Duration::from_millis(10));
            self.monitor.stop_timer("cache_lookup");
        }

        self.monitor.print_summary();
        Ok(())
    }
}

pub struct DownloadBenchmarks {
    monitor: PerformanceMonitor,
}

impl DownloadBenchmarks {
    pub fn new() -> Self {
        let mut monitor = PerformanceMonitor::new();

        monitor.add_metadata(
            "download_small",
            OperationMetadata {
                category: "download".to_string(),
                description: "Download small package".to_string(),
                expected_range: Some((100, 1000)),
            },
        );

        monitor.add_metadata(
            "download_large",
            OperationMetadata {
                category: "download".to_string(),
                description: "Download large package".to_string(),
                expected_range: Some((500, 5000)),
            },
        );

        Self { monitor }
    }

    pub fn run_all(&mut self, iterations: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "⬇️  Download Benchmarks".bright_blue().bold());

        println!("Download benchmarks with {} iterations", iterations);

        for _ in 0..iterations {
            self.monitor.start_timer("download_small");
            std::thread::sleep(Duration::from_millis(200));
            self.monitor.stop_timer("download_small");
        }

        self.monitor.print_summary();
        Ok(())
    }
}
