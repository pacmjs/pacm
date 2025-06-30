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
        let mut cache = self.index.lock().await;
        if !cache.is_empty() {
            return Ok(());
        }

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
                let chunk_size = 100.min(entries.len().max(1));
                let chunks: Vec<_> = entries.chunks(chunk_size).collect();
                let mut all_cached_packages = Vec::new();

                for chunk in chunks {
                    let mut chunk_packages = Vec::new();

                    for entry in chunk {
                        let dir_name = entry.file_name();
                        if let Some(name_str) = dir_name.to_str() {
                            if let Some((pkg_name, version, hash)) = self.parse_entry_name(name_str)
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

                                        chunk_packages.push((
                                            format!("{}@{}", pkg_name, version),
                                            cached_pkg,
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    all_cached_packages.extend(chunk_packages);
                }

                for (key, cached_pkg) in all_cached_packages {
                    cache.insert(key, cached_pkg);
                }
            }
            Err(_) => {}
        }

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

    fn parse_entry_name(&self, name: &str) -> Option<(String, String, String)> {
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

    pub async fn get(&self, key: &str) -> Option<CachedPackage> {
        let cache = self.index.lock().await;
        cache.get(key).cloned()
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
