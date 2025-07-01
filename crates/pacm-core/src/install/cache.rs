use std::collections::HashMap;
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

                let mut cache = self.index.lock().await;

                for entry in entries {
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

                                    cache.insert(format!("{}@{}", pkg_name, version), cached_pkg);
                                }
                            }
                        }
                    }
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

    pub async fn are_all_cached(&self, packages: &[(String, String)]) -> bool {
        let cache = self.index.lock().await;
        packages.iter().all(|(name, version)| {
            let key = format!("{}@{}", name, version);
            cache.contains_key(&key)
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
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}
