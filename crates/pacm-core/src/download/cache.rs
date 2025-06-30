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

        if let Ok(entries) = std::fs::read_dir(&npm_dir) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name();
                if let Some(name_str) = dir_name.to_str() {
                    if let Some((pkg_name, version, _hash)) = parse_entry_name(name_str) {
                        let store_path = entry.path();
                        if store_path.is_dir() && store_path.join("package").exists() {
                            let key = format!("{}@{}", pkg_name, version);
                            cache.insert(key, store_path);
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
}

pub fn parse_entry_name(name: &str) -> Option<(String, String, String)> {
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
