use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use pacm_constants::POPULAR_PACKAGES;
use pacm_error::Result;
use pacm_resolver::ResolvedPackage;

pub struct DependencyOptimizer {
    preload_cache: Arc<RwLock<HashMap<String, Vec<ResolvedPackage>>>>,
}

impl DependencyOptimizer {
    pub fn new() -> Self {
        Self {
            preload_cache: Arc::new(RwLock::new(HashMap::with_capacity(2000))),
        }
    }

    pub async fn preload_popular_packages(&self, client: Arc<reqwest::Client>) -> Result<()> {
        let popular_packages = POPULAR_PACKAGES.to_vec();
        if popular_packages.is_empty() {
            return Ok(());
        }

        let preload_tasks: Vec<_> = popular_packages
            .iter()
            .map(|&pkg_name| {
                let client_clone = client.clone();
                let cache = self.preload_cache.clone();

                async move {
                    if let Ok(pkg_data) =
                        pacm_registry::fetch_package_info_async(client_clone, pkg_name).await
                    {
                        if let Some(latest_version) = pkg_data.dist_tags.get("latest") {
                            let key = format!("{}@latest", pkg_name);
                            let resolved_pkg = ResolvedPackage {
                                name: pkg_name.to_string(),
                                version: latest_version.clone(),
                                resolved: format!(
                                    "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                    pkg_name, pkg_name, latest_version
                                ),
                                integrity: String::new(),
                                dependencies: HashMap::new(),
                            };

                            let mut cache_write = cache.write().await;
                            cache_write.insert(key, vec![resolved_pkg]);
                        }
                    }
                }
            })
            .collect();

        join_all(preload_tasks).await;
        Ok(())
    }

    pub async fn get_preloaded(&self, package_key: &str) -> Option<Vec<ResolvedPackage>> {
        let cache_read = self.preload_cache.read().await;
        cache_read.get(package_key).cloned()
    }

    pub async fn clear_cache(&self) {
        let mut cache_write = self.preload_cache.write().await;
        cache_write.clear();
    }
}

impl Default for DependencyOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
