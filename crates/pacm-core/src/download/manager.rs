use futures::future::join_all;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::ResolvedPackage;

use super::cache::CacheIndex;
use super::client::DownloadClient;
use super::storage::PackageStorage;

pub struct PackageDownloader {
    cache: CacheIndex,
    client: DownloadClient,
}

impl PackageDownloader {
    pub fn new() -> Self {
        Self {
            cache: CacheIndex::new(),
            client: DownloadClient::new(),
        }
    }

    pub async fn download_parallel(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<HashMap<String, (ResolvedPackage, PathBuf)>> {
        if packages.is_empty() {
            return Ok(HashMap::new());
        }

        self.cache.build(debug).await?;

        pacm_logger::status(&format!(
            "Processing {} packages with cache-first strategy...",
            packages.len()
        ));

        let stored_packages = Arc::new(Mutex::new(HashMap::new()));
        let processed = Arc::new(Mutex::new(std::collections::HashSet::new()));

        let mut cached_packages = Vec::new();
        let mut packages_to_download = Vec::new();

        for pkg in packages {
            let key = format!("{}@{}", pkg.name, pkg.version);
            if let Some(store_path) = self.cache.get(&key).await {
                cached_packages.push((pkg.clone(), store_path));
                pacm_logger::debug(&format!("Cache hit: {}", key), debug);
            } else {
                packages_to_download.push(pkg.clone());
            }
        }

        if !cached_packages.is_empty() {
            let mut stored = stored_packages.lock().await;
            for (pkg, store_path) in cached_packages {
                let key = format!("{}@{}", pkg.name, pkg.version);
                stored.insert(key, (pkg, store_path));
            }
            pacm_logger::status(&format!(
                "{} packages linked from cache instantly",
                stored.len()
            ));
        }

        if !packages_to_download.is_empty() {
            pacm_logger::status(&format!(
                "Downloading {} packages...",
                packages_to_download.len()
            ));

            let download_tasks: Vec<_> = packages_to_download
                .iter()
                .map(|pkg| {
                    let client = &self.client;
                    let stored_packages = stored_packages.clone();
                    let processed = processed.clone();
                    let pkg = pkg.clone();

                    async move {
                        let key = format!("{}@{}", pkg.name, pkg.version);

                        {
                            let mut proc = processed.lock().await;
                            if proc.contains(&key) {
                                return Ok::<(), PackageManagerError>(());
                            }
                            proc.insert(key.clone());
                        }

                        if debug {
                            pacm_logger::debug(&format!("Downloading: {}", key), debug);
                        }

                        let tarball_bytes = client.download_tarball(&pkg, debug).await?;
                        let store_path = PackageStorage::store(&pkg, &tarball_bytes, debug)?;

                        {
                            let mut stored = stored_packages.lock().await;
                            stored.insert(key, (pkg, store_path));
                        }

                        Ok(())
                    }
                })
                .collect();

            let results = join_all(download_tasks).await;

            for result in results {
                if let Err(e) = result {
                    return Err(e);
                }
            }
        }

        let stored = stored_packages.lock().await;
        let total_cached = stored.len() - packages_to_download.len();
        if total_cached > 0 {
            pacm_logger::status(&format!(
                "Total: {} from cache, {} downloaded",
                total_cached,
                packages_to_download.len()
            ));
        }
        Ok(stored.clone())
    }

    pub fn download_packages(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<HashMap<String, (ResolvedPackage, PathBuf)>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.download_parallel(packages, debug))
    }

    pub fn check_exists(&self, pkg: &ResolvedPackage, debug: bool) -> Result<Option<PathBuf>> {
        PackageStorage::check_exists(pkg, debug)
    }
}

impl Default for PackageDownloader {
    fn default() -> Self {
        Self::new()
    }
}
