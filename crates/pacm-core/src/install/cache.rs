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
            Ok(entries) => {
                let entries: Vec<_> = entries.flatten().collect();

                if entries.is_empty() {
                    return Ok(());
                }

                let cached_packages: Vec<_> = entries
                    .par_iter()
                    .filter_map(|entry| {
                        let dir_name = entry.file_name();
                        if let Some(name_str) = dir_name.to_str() {
                            if let Some((pkg_name, version, hash)) =
                                Self::parse_entry_name_static(name_str)
                            {
                                let store_path = entry.path();
                                if store_path.is_dir() {
                                    let package_dir = store_path.join("package");
                                    if package_dir.exists() {
                                        let cached_pkg = CachedPackage {
                                            name: pkg_name.clone(),
                                            version: version.clone(),
                                            resolved: format!(
                                                "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                                pkg_name, pkg_name, version
                                            ),
                                            integrity: format!("sha256-{}", hash),
                                            store_path,
                                        };

                                        Some((format!("{}@{}", pkg_name, version), cached_pkg))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
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

    fn parse_entry_name_static(name: &str) -> Option<(String, String, String)> {
        if let Some(at_pos) = name.find('@') {
            let pkg_part = &name[..at_pos];
            let rest = &name[at_pos + 1..];

            if let Some(dash_pos) = rest.find('-') {
                let version = &rest[..dash_pos];
                let hash = &rest[dash_pos + 1..];

                let pkg_name = if pkg_part.contains("_at_") {
                    pkg_part.replace("_at_", "@").replace("_slash_", "/")
                } else {
                    pkg_part.to_string()
                };

                return Some((pkg_name, version.to_string(), hash.to_string()));
            }
        }
        None
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
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}
