use super::cache::CacheManager;
use super::types::CachedPackage;
use pacm_error::Result;
use pacm_logger;
use pacm_symcap::SystemCapabilities;

pub struct FastPathAnalyzer {
    cache: CacheManager,
}

#[derive(Debug, Clone)]
pub enum InstallationPath {
    InstantLink {
        cached_packages: Vec<CachedPackage>,
        skip_dependency_check: bool,
    },
    CachedWithDeps {
        main_package: CachedPackage,
        need_dep_resolution: bool,
    },
    OptimizedDownload {
        can_skip_transitive: bool,
        estimated_complexity: usize,
    },
    FullResolution,
}

impl FastPathAnalyzer {
    pub fn new(cache: CacheManager) -> Self {
        Self { cache }
    }

    pub async fn analyze_single_package(
        &self,
        name: &str,
        version_range: &str,
        debug: bool,
    ) -> Result<InstallationPath> {
        let cache_key = format!("{}@{}", name, version_range);

        if let Some(cached_package) = self.cache.get(&cache_key).await {
            if self.is_simple_package(&cached_package, debug).await {
                if debug {
                    pacm_logger::debug(
                        &format!("Package {} identified as simple - using instant link", name),
                        debug,
                    );
                }
                return Ok(InstallationPath::InstantLink {
                    cached_packages: vec![cached_package],
                    skip_dependency_check: true,
                });
            } else {
                return Ok(InstallationPath::CachedWithDeps {
                    main_package: cached_package,
                    need_dep_resolution: true,
                });
            }
        }

        if self.is_likely_simple_package(name) {
            Ok(InstallationPath::OptimizedDownload {
                can_skip_transitive: true,
                estimated_complexity: 1,
            })
        } else if self.is_known_complex_package(name) {
            Ok(InstallationPath::FullResolution)
        } else {
            Ok(InstallationPath::OptimizedDownload {
                can_skip_transitive: false,
                estimated_complexity: 5,
            })
        }
    }

    pub async fn analyze_bulk_install(
        &self,
        packages: &[(String, String)],
        debug: bool,
    ) -> Result<BulkInstallationStrategy> {
        let system_caps = SystemCapabilities::get();
        let mut instant_packages = Vec::new();
        let mut cached_packages = Vec::new();
        let mut download_packages = Vec::new();
        let mut complex_packages = Vec::new();

        let batch_size = system_caps.get_network_batch_size(packages.len());
        let batches: Vec<_> = packages.chunks(batch_size).collect();

        for batch in batches {
            let cache_checks: Vec<_> = batch
                .iter()
                .map(|(name, version)| {
                    let cache_key = format!("{}@{}", name, version);
                    async move {
                        (
                            name.clone(),
                            version.clone(),
                            self.cache.get(&cache_key).await,
                        )
                    }
                })
                .collect();

            let results = futures::future::join_all(cache_checks).await;

            for (name, version, cached_opt) in results {
                if let Some(cached) = cached_opt {
                    if system_caps.should_skip_transitive_analysis(&name)
                        || self.is_likely_instant_package(&name)
                    {
                        instant_packages.push((name, version, cached));
                    } else if self.is_simple_package_fast(&cached).await {
                        instant_packages.push((name, version, cached));
                    } else {
                        cached_packages.push((name, version, cached));
                    }
                } else if self.is_known_complex_package(&name) {
                    complex_packages.push((name, version));
                } else {
                    download_packages.push((name, version));
                }
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Bulk analysis: {} instant, {} cached, {} download, {} complex",
                    instant_packages.len(),
                    cached_packages.len(),
                    download_packages.len(),
                    complex_packages.len()
                ),
                debug,
            );
        }

        Ok(BulkInstallationStrategy {
            instant_packages,
            cached_packages,
            download_packages,
            complex_packages,
        })
    }

    async fn is_simple_package(&self, cached_package: &CachedPackage, debug: bool) -> bool {
        let package_json_path = cached_package
            .store_path
            .join("package")
            .join("package.json");

        if !package_json_path.exists() {
            return true; // No package.json = simple
        }

        match std::fs::read_to_string(&package_json_path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(pkg_data) => {
                        let deps = pkg_data
                            .get("dependencies")
                            .and_then(|d| d.as_object())
                            .map(|deps| deps.len())
                            .unwrap_or(0);

                        let optional_deps = pkg_data
                            .get("optionalDependencies")
                            .and_then(|d| d.as_object())
                            .map(|deps| deps.len())
                            .unwrap_or(0);

                        // Consider simple if has 3 or fewer total dependencies
                        let is_simple = (deps + optional_deps) <= 3;

                        if debug && is_simple {
                            pacm_logger::debug(
                                &format!(
                                    "Package {} has {} deps - considered simple",
                                    cached_package.name,
                                    deps + optional_deps
                                ),
                                debug,
                            );
                        }

                        is_simple
                    }
                    Err(_) => true, // Can't parse = assume simple
                }
            }
            Err(_) => true, // Can't read = assume simple
        }
    }

    async fn is_simple_package_fast(&self, cached_package: &CachedPackage) -> bool {
        self.is_likely_instant_package(&cached_package.name)
    }

    fn is_likely_instant_package(&self, name: &str) -> bool {
        const INSTANT_PACKAGES: &[&str] = &[
            "ms",
            "debug",
            "semver",
            "uuid",
            "chalk",
            "colors",
            "rimraf",
            "mkdirp",
            "glob",
            "lodash",
            "underscore",
        ];

        INSTANT_PACKAGES.contains(&name)
            || name.starts_with("@types/")
            || name.len() < 5
            || name.contains("-util")
            || name.contains("-tool")
    }

    fn is_likely_simple_package(&self, name: &str) -> bool {
        const SIMPLE_PACKAGES: &[&str] = &[
            "lodash",
            "underscore",
            "moment",
            "uuid",
            "chalk",
            "colors",
            "debug",
            "ms",
            "semver",
            "rimraf",
            "mkdirp",
            "glob",
            "commander",
            "yargs",
            "inquirer",
            "ora",
            "cli-progress",
            "axios",
            "node-fetch",
            "request",
            "cheerio",
            "jsdom",
        ];

        SIMPLE_PACKAGES.contains(&name)
            || name.starts_with("@types/")
            || name.contains("-utils")
            || name.contains("-helper")
            || name.contains("-tool")
    }

    fn is_known_complex_package(&self, name: &str) -> bool {
        const COMPLEX_PACKAGES: &[&str] = &[
            "react",
            "vue",
            "angular",
            "express",
            "webpack",
            "rollup",
            "babel",
            "typescript",
            "eslint",
            "jest",
            "mocha",
            "cypress",
            "next",
            "nuxt",
            "gatsby",
            "create-react-app",
            "vite",
        ];

        COMPLEX_PACKAGES.contains(&name)
            || name.starts_with("@babel/")
            || name.starts_with("@angular/")
            || name.starts_with("@types/react")
            || name.contains("webpack")
            || name.contains("babel")
    }
}

#[derive(Debug)]
pub struct BulkInstallationStrategy {
    pub instant_packages: Vec<(String, String, CachedPackage)>,
    pub cached_packages: Vec<(String, String, CachedPackage)>,
    pub download_packages: Vec<(String, String)>,
    pub complex_packages: Vec<(String, String)>,
}

impl BulkInstallationStrategy {
    pub fn total_packages(&self) -> usize {
        self.instant_packages.len()
            + self.cached_packages.len()
            + self.download_packages.len()
            + self.complex_packages.len()
    }

    pub fn can_use_fast_path(&self) -> bool {
        let fast_count = self.instant_packages.len() + self.cached_packages.len();
        let total = self.total_packages();

        if total == 0 {
            return false;
        }

        (fast_count * 100 / total) >= 80
    }
}
