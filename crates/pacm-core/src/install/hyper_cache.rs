use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::cache::CacheManager;
use pacm_logger;
use pacm_symcap::SystemCapabilities;

pub struct HyperCache {
    simple_packages: Arc<RwLock<HashSet<String>>>,
    complex_packages: Arc<RwLock<HashSet<String>>>,
    instant_packages: Arc<RwLock<HashSet<String>>>,
    dependency_count_cache: Arc<RwLock<HashMap<String, usize>>>,
    package_resolution_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl HyperCache {
    pub fn new() -> Self {
        Self {
            simple_packages: Arc::new(RwLock::new(HashSet::new())),
            complex_packages: Arc::new(RwLock::new(HashSet::new())),
            instant_packages: Arc::new(RwLock::new(HashSet::new())),
            dependency_count_cache: Arc::new(RwLock::new(HashMap::new())),
            package_resolution_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn warm_up(&self, cache_manager: &CacheManager, debug: bool) {
        let system_caps = SystemCapabilities::get();

        if debug {
            pacm_logger::debug("Warming up package categorization cache...", debug);
        }

        let known_simple = vec![
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

        let known_complex = vec![
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
        ];

        let known_instant = vec![
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

        {
            let mut simple_cache = self.simple_packages.write().await;
            for pkg in known_simple {
                simple_cache.insert(pkg.to_string());
            }
        }

        {
            let mut complex_cache = self.complex_packages.write().await;
            for pkg in known_complex {
                complex_cache.insert(pkg.to_string());
            }
        }

        {
            let mut instant_cache = self.instant_packages.write().await;
            for pkg in known_instant {
                instant_cache.insert(pkg.to_string());
            }
        }

        if system_caps.available_memory_gb > 4.0 {
            self.pre_analyze_cached_packages(cache_manager, debug).await;
        }
    }

    async fn pre_analyze_cached_packages(&self, cache_manager: &CacheManager, debug: bool) {
        let cache_stats = cache_manager.get_stats().await;
        let cache_size = cache_stats.0;

        if cache_size > 100 {
            let sample_size = (cache_size / 10).min(50).max(10);

            if debug {
                pacm_logger::debug(
                    &format!(
                        "Pre-analyzing {} cached packages for dependency counts",
                        sample_size
                    ),
                    debug,
                );
            }
        }
    }

    pub async fn is_simple_package(&self, package_name: &str) -> Option<bool> {
        {
            let instant_cache = self.instant_packages.read().await;
            if instant_cache.contains(package_name) {
                return Some(true);
            }
        }

        {
            let simple_cache = self.simple_packages.read().await;
            if simple_cache.contains(package_name) {
                return Some(true);
            }
        }

        {
            let complex_cache = self.complex_packages.read().await;
            if complex_cache.contains(package_name) {
                return Some(false);
            }
        }

        if self.heuristic_simple_check(package_name) {
            let mut simple_cache = self.simple_packages.write().await;
            simple_cache.insert(package_name.to_string());
            return Some(true);
        }

        if self.heuristic_complex_check(package_name) {
            let mut complex_cache = self.complex_packages.write().await;
            complex_cache.insert(package_name.to_string());
            return Some(false);
        }

        None // Unknown, needs full analysis
    }

    fn heuristic_simple_check(&self, package_name: &str) -> bool {
        package_name.starts_with("@types/")
            || package_name.contains("-utils")
            || package_name.contains("-helper")
            || package_name.contains("-tool")
            || package_name.contains("-cli")
            || package_name.len() < 6
    }

    fn heuristic_complex_check(&self, package_name: &str) -> bool {
        package_name.starts_with("@babel/")
            || package_name.starts_with("@angular/")
            || package_name.starts_with("@webpack/")
            || package_name.contains("webpack")
            || package_name.contains("babel")
            || package_name.contains("eslint")
            || package_name.contains("typescript")
    }

    pub async fn get_dependency_count(&self, package_name: &str) -> Option<usize> {
        let dep_cache = self.dependency_count_cache.read().await;
        dep_cache.get(package_name).copied()
    }

    pub async fn cache_dependency_count(&self, package_name: &str, count: usize) {
        let mut dep_cache = self.dependency_count_cache.write().await;
        dep_cache.insert(package_name.to_string(), count);
    }

    pub async fn clear_all(&self) {
        {
            let mut simple_cache = self.simple_packages.write().await;
            simple_cache.clear();
        }

        {
            let mut complex_cache = self.complex_packages.write().await;
            complex_cache.clear();
        }

        {
            let mut instant_cache = self.instant_packages.write().await;
            instant_cache.clear();
        }

        {
            let mut dep_cache = self.dependency_count_cache.write().await;
            dep_cache.clear();
        }

        {
            let mut resolution_cache = self.package_resolution_cache.write().await;
            resolution_cache.clear();
        }
    }

    pub async fn get_cache_stats(&self) -> (usize, usize, usize, usize) {
        let simple_count = {
            let cache = self.simple_packages.read().await;
            cache.len()
        };

        let complex_count = {
            let cache = self.complex_packages.read().await;
            cache.len()
        };

        let instant_count = {
            let cache = self.instant_packages.read().await;
            cache.len()
        };

        let dep_count = {
            let cache = self.dependency_count_cache.read().await;
            cache.len()
        };

        (simple_count, complex_count, instant_count, dep_count)
    }
}

impl Default for HyperCache {
    fn default() -> Self {
        Self::new()
    }
}
