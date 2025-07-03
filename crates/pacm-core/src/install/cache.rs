use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::CachedPackage;
use pacm_error::Result;
use pacm_logger;
use pacm_store::get_store_path;

pub struct CacheManager {
    index: Arc<Mutex<HashMap<String, CachedPackage>>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn build_index(&self, debug: bool) -> Result<()> {
        let cache = self.index.lock().await;
        if !cache.is_empty() {
            return Ok(());
        }
        drop(cache); // Release lock early

        let store_base = get_store_path();
        let npm_dir = store_base.join("npm");

        if !npm_dir.exists() {
            return Ok(());
        }

        pacm_logger::debug("Building cache index...", debug);
        let start = std::time::Instant::now();

        match std::fs::read_dir(&npm_dir) {
            Ok(package_entries) => {
                let package_entries: Vec<_> = package_entries.flatten().collect();

                if package_entries.is_empty() {
                    return Ok(());
                }

                let cached_packages: Vec<_> = package_entries
                    .par_iter()
                    .filter_map(|package_entry| {
                        if package_entry.file_type().ok()?.is_dir() {
                            let package_name = Self::unsanitize_package_name(
                                &package_entry.file_name().to_string_lossy(),
                            );

                            if let Ok(version_entries) = std::fs::read_dir(package_entry.path()) {
                                let versions: Vec<_> = version_entries
                                    .flatten()
                                    .filter_map(|version_entry| {
                                        if version_entry.file_type().ok()?.is_dir() {
                                            let version = version_entry
                                                .file_name()
                                                .to_string_lossy()
                                                .to_string();
                                            let store_path = version_entry.path();
                                            let package_dir = store_path.join("package");

                                            if package_dir.exists() {
                                                let cached_pkg = CachedPackage {
                                                    name: package_name.clone(),
                                                    version: version.clone(),
                                                    resolved: format!(
                                                        "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                                        package_name, package_name, version
                                                    ),
                                                    integrity: String::new(), // We no longer store hash in path
                                                    store_path,
                                                };

                                                Some((
                                                    format!("{}@{}", package_name, version),
                                                    cached_pkg,
                                                ))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();

                                Some(versions)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect();

                let mut cache = self.index.lock().await;
                for (key, cached_pkg) in cached_packages {
                    cache.insert(key, cached_pkg);
                }
            }
            Err(_) => {}
        }

        let cache = self.index.lock().await;
        let duration = start.elapsed();
        pacm_logger::debug(
            &format!(
                "Cache index built with {} entries in {:?}",
                cache.len(),
                duration
            ),
            debug,
        );

        Ok(())
    }

    pub async fn get(&self, key: &str) -> Option<CachedPackage> {
        let cache = self.index.lock().await;
        cache.get(key).cloned()
    }

    pub async fn get_batch(&self, keys: &[String]) -> Vec<(String, Option<CachedPackage>)> {
        let cache = self.index.lock().await;
        keys.iter()
            .map(|key| (key.clone(), cache.get(key).cloned()))
            .collect()
    }

    pub async fn get_batch_direct(&self, deps: &[(String, String)]) -> Vec<Option<CachedPackage>> {
        let cache = self.index.lock().await;
        deps.iter()
            .map(|(name, version_range)| {
                let key = format!("{}@{}", name, version_range);
                if let Some(cached) = cache.get(&key) {
                    return Some(cached.clone());
                }

                if version_range == "latest"
                    || version_range.is_empty()
                    || (!version_range.chars().next().unwrap_or('0').is_ascii_digit())
                {
                    let name_prefix = format!("{}@", name);

                    let versions: Vec<_> = cache
                        .iter()
                        .filter(|(key, _)| key.starts_with(&name_prefix))
                        .map(|(_, cached_pkg)| cached_pkg)
                        .collect();

                    if let Some(cached_pkg) = versions.first() {
                        return Some((*cached_pkg).clone());
                    }
                }

                None
            })
            .collect()
    }

    pub async fn are_all_cached(&self, packages: &[(String, String)]) -> bool {
        let cache = self.index.lock().await;
        packages.iter().all(|(name, version_range)| {
            if version_range.chars().next().unwrap_or('0').is_ascii_digit()
                && !version_range.contains('^')
                && !version_range.contains('~')
                && !version_range.contains('*')
            {
                let key = format!("{}@{}", name, version_range);
                return cache.contains_key(&key);
            }

            let name_prefix = format!("{}@", name);
            cache.keys().any(|key| key.starts_with(&name_prefix))
        })
    }

    pub async fn get_stats(&self) -> (usize, f64) {
        let cache = self.index.lock().await;
        let size = cache.len();
        let memory_usage = size as f64 * 0.5; // Rough estimate in KB
        (size, memory_usage)
    }

    pub async fn contains(&self, key: &str) -> bool {
        let cache = self.index.lock().await;
        cache.contains_key(key)
    }

    pub async fn len(&self) -> usize {
        let cache = self.index.lock().await;
        cache.len()
    }

    pub async fn find_versions_for_package(&self, package_name: &str) -> Vec<(String, PathBuf)> {
        let cache = self.index.lock().await;
        cache
            .values()
            .filter(|cached_pkg| cached_pkg.name == package_name)
            .map(|cached_pkg| (cached_pkg.version.clone(), cached_pkg.store_path.clone()))
            .collect()
    }

    fn unsanitize_package_name(safe_name: &str) -> String {
        safe_name.replace("_at_", "@").replace("_slash_", "/")
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}
