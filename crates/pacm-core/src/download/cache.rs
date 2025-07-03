use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use pacm_error::Result;
use pacm_logger;

pub struct CacheIndex {
    index: Arc<Mutex<HashMap<String, PathBuf>>>,
}

impl CacheIndex {
    pub fn new() -> Self {
        Self {
            index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn build(&self, debug: bool) -> Result<()> {
        let mut cache = self.index.lock().await;
        if !cache.is_empty() {
            return Ok(()); // Already built
        }

        let store_base = pacm_store::get_store_path();
        let npm_dir = store_base.join("npm");

        if !npm_dir.exists() {
            return Ok(());
        }

        pacm_logger::debug("Building cache index...", debug);
        let start = std::time::Instant::now();

        if let Ok(package_entries) = std::fs::read_dir(&npm_dir) {
            for package_entry in package_entries.flatten() {
                if package_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    let package_name =
                        Self::unsanitize_package_name(&package_entry.file_name().to_string_lossy());

                    if let Ok(version_entries) = std::fs::read_dir(package_entry.path()) {
                        for version_entry in version_entries.flatten() {
                            if version_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                                let version =
                                    version_entry.file_name().to_string_lossy().to_string();
                                let package_dir = version_entry.path().join("package");

                                if package_dir.exists() {
                                    let key = format!("{}@{}", package_name, version);
                                    cache.insert(key, version_entry.path());
                                }
                            }
                        }
                    }
                }
            }
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

    pub async fn get(&self, key: &str) -> Option<PathBuf> {
        let cache = self.index.lock().await;
        cache.get(key).cloned()
    }

    fn unsanitize_package_name(safe_name: &str) -> String {
        safe_name.replace("_at_", "@").replace("_slash_", "/")
    }

    pub async fn find_versions_for_package(&self, package_name: &str) -> Vec<(String, PathBuf)> {
        let cache = self.index.lock().await;
        let name_prefix = format!("{}@", package_name);

        cache
            .iter()
            .filter(|(key, _)| key.starts_with(&name_prefix))
            .map(|(key, path)| {
                let version = &key[name_prefix.len()..];
                (version.to_string(), path.clone())
            })
            .collect()
    }
}
