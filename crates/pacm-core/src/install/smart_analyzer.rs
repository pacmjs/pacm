use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::cache::CacheManager;
use super::types::CachedPackage;
use pacm_error::Result;
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_symcap::SystemCapabilities;

pub struct SmartDependencyAnalyzer {
    cache: CacheManager,
    simple_package_cache: Arc<Mutex<HashSet<String>>>,
    complex_package_cache: Arc<Mutex<HashSet<String>>>,
    resolution_cache: Arc<Mutex<HashMap<String, AnalysisResult>>>,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub complexity: PackageComplexity,
    pub estimated_dependencies: usize,
    pub can_skip_transitive: bool,
    pub cached_result: Option<Vec<ResolvedPackage>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageComplexity {
    /// No dependencies, instant installation
    Trivial,
    /// 1-3 dependencies, minimal analysis needed
    Simple,
    /// 4-10 dependencies, moderate analysis
    Moderate,
    /// 11+ dependencies or known complex framework
    Complex,
}

impl SmartDependencyAnalyzer {
    pub fn new(cache: CacheManager) -> Self {
        Self {
            cache,
            simple_package_cache: Arc::new(Mutex::new(HashSet::new())),
            complex_package_cache: Arc::new(Mutex::new(HashSet::new())),
            resolution_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn analyze_packages(
        &self,
        packages: &[(String, String)],
        debug: bool,
    ) -> Result<Vec<AnalysisResult>> {
        let system_caps = SystemCapabilities::get();
        let mut results = Vec::with_capacity(packages.len());

        let cache_hits = self.check_resolution_cache(packages).await;

        for (i, (name, version)) in packages.iter().enumerate() {
            if let Some(cached_result) = &cache_hits[i] {
                results.push(cached_result.clone());
                continue;
            }

            let analysis = if system_caps.should_skip_transitive_analysis(name) {
                AnalysisResult {
                    complexity: PackageComplexity::Simple,
                    estimated_dependencies: 1,
                    can_skip_transitive: true,
                    cached_result: None,
                }
            } else {
                self.analyze_single_package(name, version, debug).await?
            };

            let cache_key = format!("{}@{}", name, version);
            let mut cache = self.resolution_cache.lock().await;
            cache.insert(cache_key, analysis.clone());

            results.push(analysis);
        }

        Ok(results)
    }

    async fn check_resolution_cache(
        &self,
        packages: &[(String, String)],
    ) -> Vec<Option<AnalysisResult>> {
        let cache = self.resolution_cache.lock().await;
        packages
            .iter()
            .map(|(name, version)| {
                let key = format!("{}@{}", name, version);
                cache.get(&key).cloned()
            })
            .collect()
    }

    async fn analyze_single_package(
        &self,
        name: &str,
        version: &str,
        debug: bool,
    ) -> Result<AnalysisResult> {
        {
            let simple_cache = self.simple_package_cache.lock().await;
            if simple_cache.contains(name) {
                return Ok(AnalysisResult {
                    complexity: PackageComplexity::Simple,
                    estimated_dependencies: 1,
                    can_skip_transitive: true,
                    cached_result: None,
                });
            }
        }

        {
            let complex_cache = self.complex_package_cache.lock().await;
            if complex_cache.contains(name) {
                return Ok(AnalysisResult {
                    complexity: PackageComplexity::Complex,
                    estimated_dependencies: 20,
                    can_skip_transitive: false,
                    cached_result: None,
                });
            }
        }

        let cache_key = format!("{}@{}", name, version);
        if let Some(cached_pkg) = self.cache.get(&cache_key).await {
            let analysis = self.analyze_cached_package(&cached_pkg, debug).await;

            match analysis.complexity {
                PackageComplexity::Trivial | PackageComplexity::Simple => {
                    let mut simple_cache = self.simple_package_cache.lock().await;
                    simple_cache.insert(name.to_string());
                }
                PackageComplexity::Complex => {
                    let mut complex_cache = self.complex_package_cache.lock().await;
                    complex_cache.insert(name.to_string());
                }
                _ => {} // Don't cache moderate packages
            }

            return Ok(analysis);
        }

        Ok(self.heuristic_analysis(name))
    }

    async fn analyze_cached_package(
        &self,
        cached_pkg: &CachedPackage,
        debug: bool,
    ) -> AnalysisResult {
        let package_json_path = cached_pkg.store_path.join("package").join("package.json");

        if !package_json_path.exists() {
            return AnalysisResult {
                complexity: PackageComplexity::Trivial,
                estimated_dependencies: 0,
                can_skip_transitive: true,
                cached_result: None,
            };
        }

        match std::fs::read_to_string(&package_json_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(pkg_data) => {
                    let deps_count = pkg_data
                        .get("dependencies")
                        .and_then(|d| d.as_object())
                        .map(|deps| deps.len())
                        .unwrap_or(0);

                    let optional_deps_count = pkg_data
                        .get("optionalDependencies")
                        .and_then(|d| d.as_object())
                        .map(|deps| deps.len())
                        .unwrap_or(0);

                    let dev_deps_count = pkg_data
                        .get("devDependencies")
                        .and_then(|d| d.as_object())
                        .map(|deps| deps.len())
                        .unwrap_or(0);

                    let total_deps = deps_count + optional_deps_count;

                    let complexity = match total_deps {
                        0 => PackageComplexity::Trivial,
                        1..=3 => PackageComplexity::Simple,
                        4..=10 => PackageComplexity::Moderate,
                        _ => PackageComplexity::Complex,
                    };

                    let has_scripts = pkg_data
                        .get("scripts")
                        .and_then(|s| s.as_object())
                        .map(|scripts| scripts.len() > 3)
                        .unwrap_or(false);

                    let has_many_dev_deps = dev_deps_count > 10;

                    let final_complexity = if has_scripts || has_many_dev_deps {
                        match complexity {
                            PackageComplexity::Trivial => PackageComplexity::Simple,
                            PackageComplexity::Simple => PackageComplexity::Moderate,
                            other => other,
                        }
                    } else {
                        complexity
                    };

                    if debug && total_deps > 0 {
                        pacm_logger::debug(
                            &format!(
                                "Package {} has {} deps ({} optional) - complexity: {:?}",
                                cached_pkg.name, deps_count, optional_deps_count, final_complexity
                            ),
                            debug,
                        );
                    }

                    AnalysisResult {
                        complexity: final_complexity.clone(),
                        estimated_dependencies: total_deps,
                        can_skip_transitive: matches!(
                            final_complexity,
                            PackageComplexity::Trivial | PackageComplexity::Simple
                        ),
                        cached_result: None,
                    }
                }
                Err(_) => AnalysisResult {
                    complexity: PackageComplexity::Simple,
                    estimated_dependencies: 1,
                    can_skip_transitive: true,
                    cached_result: None,
                },
            },
            Err(_) => AnalysisResult {
                complexity: PackageComplexity::Trivial,
                estimated_dependencies: 0,
                can_skip_transitive: true,
                cached_result: None,
            },
        }
    }

    fn heuristic_analysis(&self, name: &str) -> AnalysisResult {
        const COMPLEX_PACKAGES: &[&str] = &[
            "react",
            "vue",
            "angular",
            "express",
            "webpack",
            "rollup",
            "vite",
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
            "@angular/core",
            "karma",
            "protractor",
            "storybook",
            "@storybook/react",
        ];

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
            "fs-extra",
            "path",
            "util",
            "events",
            "stream",
            "crypto",
        ];

        if COMPLEX_PACKAGES.iter().any(|&pkg| name.contains(pkg))
            || name.starts_with("@babel/")
            || name.starts_with("@angular/")
            || name.starts_with("@webpack/")
            || name.contains("webpack")
            || name.contains("babel")
            || name.contains("eslint")
        {
            return AnalysisResult {
                complexity: PackageComplexity::Complex,
                estimated_dependencies: 15,
                can_skip_transitive: false,
                cached_result: None,
            };
        }

        if SIMPLE_PACKAGES.contains(&name)
            || name.starts_with("@types/")
            || name.contains("-utils")
            || name.contains("-helper")
            || name.contains("-tool")
            || name.contains("-cli")
            || name.len() < 6
        {
            return AnalysisResult {
                complexity: PackageComplexity::Simple,
                estimated_dependencies: 2,
                can_skip_transitive: true,
                cached_result: None,
            };
        }

        AnalysisResult {
            complexity: PackageComplexity::Moderate,
            estimated_dependencies: 5,
            can_skip_transitive: false,
            cached_result: None,
        }
    }

    pub async fn clear_caches(&self) {
        let mut simple_cache = self.simple_package_cache.lock().await;
        simple_cache.clear();

        let mut complex_cache = self.complex_package_cache.lock().await;
        complex_cache.clear();

        let mut resolution_cache = self.resolution_cache.lock().await;
        resolution_cache.clear();
    }

    pub async fn get_cache_stats(&self) -> (usize, usize, usize) {
        let simple_count = {
            let simple_cache = self.simple_package_cache.lock().await;
            simple_cache.len()
        };

        let complex_count = {
            let complex_cache = self.complex_package_cache.lock().await;
            complex_cache.len()
        };

        let resolution_count = {
            let resolution_cache = self.resolution_cache.lock().await;
            resolution_cache.len()
        };

        (simple_count, complex_count, resolution_count)
    }
}
