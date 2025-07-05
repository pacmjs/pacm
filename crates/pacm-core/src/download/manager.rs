use futures::future::join_all;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_symcap::SystemCapabilities;

use super::cache::CacheIndex;
use super::client::DownloadClient;

pub struct PackageDownloader {
    cache: CacheIndex,
    client: DownloadClient,
    download_semaphore: Arc<Semaphore>,
}

impl PackageDownloader {
    pub fn new() -> Self {
        let system_caps = SystemCapabilities::get();

        Self {
            cache: CacheIndex::new(),
            client: DownloadClient::new(),
            download_semaphore: Arc::new(Semaphore::new(system_caps.optimal_parallel_downloads)),
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

        let system_caps = SystemCapabilities::get();
        let start_time = std::time::Instant::now();
        self.cache.build(debug).await?;

        if !debug {
            pacm_logger::status(&format!(
                "Downloading {} packages using {} parallel connections...",
                packages.len(),
                system_caps.optimal_parallel_downloads
            ));
        }

        let stored_packages = Arc::new(Mutex::new(HashMap::new()));
        let processed = Arc::new(Mutex::new(std::collections::HashSet::new()));

        let cache_start = std::time::Instant::now();
        let (cached_packages, packages_to_download) = self.separate_cached(packages, debug).await?;

        if debug {
            pacm_logger::debug(
                &format!(
                    "Cache separation completed in {:?} ({} cached, {} to download)",
                    cache_start.elapsed(),
                    cached_packages.len(),
                    packages_to_download.len()
                ),
                debug,
            );
        }

        if !cached_packages.is_empty() {
            let mut stored = stored_packages.lock().await;
            for (pkg, store_path) in cached_packages {
                let key = format!("{}@{}", pkg.name, pkg.version);
                stored.insert(key, (pkg, store_path));
            }
            if !debug {
                pacm_logger::debug(
                    &format!("{} packages linked from cache", stored.len()),
                    false,
                );
            }
        }

        if !packages_to_download.is_empty() {
            let download_start = std::time::Instant::now();

            let batch_size = system_caps.get_network_batch_size(packages_to_download.len());
            let batches: Vec<_> = packages_to_download.chunks(batch_size).collect();

            if debug {
                pacm_logger::debug(
                    &format!(
                        "Downloading {} packages in {} batches of up to {} packages each",
                        packages_to_download.len(),
                        batches.len(),
                        batch_size
                    ),
                    debug,
                );
            }

            for (batch_idx, batch) in batches.into_iter().enumerate() {
                if debug && batch.len() > 1 {
                    pacm_logger::debug(
                        &format!(
                            "Processing download batch {} with {} packages",
                            batch_idx + 1,
                            batch.len()
                        ),
                        debug,
                    );
                }

                let download_tasks: Vec<_> = batch
                    .iter()
                    .map(|pkg| {
                        let client = &self.client;
                        let stored_packages = stored_packages.clone();
                        let processed = processed.clone();
                        let pkg = pkg.clone();
                        let semaphore = self.download_semaphore.clone();

                        async move {
                            let _permit = semaphore.acquire().await.unwrap();

                            let key = format!("{}@{}", pkg.name, pkg.version);

                            {
                                let mut proc = processed.lock().await;
                                if proc.contains(&key) {
                                    return Ok::<(), PackageManagerError>(());
                                }
                                proc.insert(key.clone());
                            }

                            match client.download_tarball(&pkg, debug).await {
                                Ok(tarball_data) => {
                                    if let Ok(store_path) = pacm_store::store_package(
                                        &pkg.name,
                                        &pkg.version,
                                        &tarball_data,
                                    ) {
                                        let mut stored = stored_packages.lock().await;
                                        stored.insert(key.clone(), (pkg, store_path));

                                        if debug {
                                            pacm_logger::debug(
                                                &format!("Downloaded: {}", key),
                                                debug,
                                            );
                                        }
                                    } else {
                                        pacm_logger::error(&format!(
                                            "Failed to store package: {}",
                                            key
                                        ));
                                        return Err(PackageManagerError::StorageFailed(
                                            key.clone(),
                                            "Failed to store package".to_string(),
                                        ));
                                    }
                                }
                                Err(e) => {
                                    pacm_logger::error(&format!(
                                        "Failed to download {}: {}",
                                        key, e
                                    ));
                                    return Err(e);
                                }
                            }

                            Ok(())
                        }
                    })
                    .collect();

                let download_results = join_all(download_tasks).await;

                for result in download_results {
                    if let Err(e) = result {
                        return Err(e);
                    }
                }
            }

            if debug {
                pacm_logger::debug(
                    &format!("All downloads completed in {:?}", download_start.elapsed()),
                    debug,
                );
            }
        }

        let final_stored = stored_packages.lock().await.clone();

        if debug {
            pacm_logger::debug(
                &format!(
                    "Total download process completed in {:?}",
                    start_time.elapsed()
                ),
                debug,
            );
        }

        Ok(final_stored)
    }

    async fn separate_cached(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<(Vec<(ResolvedPackage, PathBuf)>, Vec<ResolvedPackage>)> {
        let mut cached_packages = Vec::new();
        let mut packages_to_download = Vec::new();

        let cache_tasks: Vec<_> = packages
            .iter()
            .map(|pkg| {
                let key = format!("{}@{}", pkg.name, pkg.version);
                let pkg_clone = pkg.clone();
                async move {
                    if let Some(store_path) = self.cache.get(&key).await {
                        (pkg_clone, Some(store_path))
                    } else {
                        (pkg_clone, None)
                    }
                }
            })
            .collect();

        let cache_results = join_all(cache_tasks).await;

        for (pkg, store_path_opt) in cache_results {
            if let Some(store_path) = store_path_opt {
                if debug {
                    pacm_logger::debug(&format!("Cache hit: {}@{}", pkg.name, pkg.version), debug);
                }
                cached_packages.push((pkg, store_path));
            } else {
                packages_to_download.push(pkg);
            }
        }

        Ok((cached_packages, packages_to_download))
    }

    pub fn download_packages(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<HashMap<String, (ResolvedPackage, PathBuf)>> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(PackageManagerError::NetworkError(
                "download_packages called from async context. Use download_parallel instead."
                    .to_string(),
            ));
        }

        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.download_parallel(packages, debug))
    }
}

impl Default for PackageDownloader {
    fn default() -> Self {
        Self::new()
    }
}
