use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::cache::CacheManager;
use super::types::CachedPackage;
use pacm_constants::USER_AGENT;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::{ResolvedPackage, resolve_full_tree_async};

pub struct DependencyResolver {
    client: Arc<reqwest::Client>,
    resolution_cache: Arc<Mutex<HashMap<String, Vec<ResolvedPackage>>>>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            client: Arc::new(
                reqwest::Client::builder()
                    .pool_max_idle_per_host(25)
                    .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
                    .timeout(std::time::Duration::from_secs(45))
                    .connect_timeout(std::time::Duration::from_secs(20))
                    .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
                    .tcp_nodelay(true)
                    .user_agent(USER_AGENT)
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new()),
            ),
            resolution_cache: Arc::new(Mutex::new(HashMap::with_capacity(1000))),
        }
    }

    pub fn get_client(&self) -> Arc<reqwest::Client> {
        self.client.clone()
    }

    pub async fn resolve_deps_optimized(
        &self,
        direct_deps: &[(String, String)],
        _use_lockfile: bool,
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashSet<String>,
        HashMap<String, ResolvedPackage>,
    )> {
        let start_time = std::time::Instant::now();

        if debug {
            if direct_deps.len() == 1 {
                pacm_logger::debug(
                    &format!("Starting fast analysis for {}", direct_deps[0].0),
                    debug,
                );
            } else {
                pacm_logger::debug(
                    &format!(
                        "Starting fast analysis for {} direct dependencies",
                        direct_deps.len()
                    ),
                    debug,
                );
            }
        }

        let cache_check_start = std::time::Instant::now();
        let direct_cache_results = cache_manager.get_batch_direct(direct_deps).await;

        if debug {
            pacm_logger::debug(
                &format!(
                    "Direct cache check completed in {:?}",
                    cache_check_start.elapsed()
                ),
                debug,
            );
        }

        let mut cached_packages = Vec::new();
        let mut packages_to_resolve = Vec::new();
        let mut direct_names = HashSet::new();
        let mut all_resolved = HashMap::new();

        for ((name, version), cached_opt) in direct_deps.iter().zip(direct_cache_results) {
            direct_names.insert(name.clone());

            if let Some(cached) = cached_opt {
                if debug {
                    pacm_logger::debug(&format!("Found {} in cache", name), debug);
                }
                cached_packages.push(cached.clone());
                let key = format!("{}@{}", cached.name, cached.version);
                let resolved_pkg = ResolvedPackage {
                    name: cached.name.clone(),
                    version: cached.version.clone(),
                    resolved: cached.resolved.clone(),
                    integrity: cached.integrity.clone(),
                    dependencies: HashMap::new(), // Will be populated if needed
                    optional_dependencies: HashMap::new(),
                    os: None,
                    cpu: None,
                };
                all_resolved.insert(key, resolved_pkg);
            } else {
                packages_to_resolve.push((name.clone(), version.clone()));
            }
        }

        let mut packages_to_download = Vec::new();

        if !packages_to_resolve.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("Resolving {} uncached packages", packages_to_resolve.len()),
                    debug,
                );
            }

            let resolve_start = std::time::Instant::now();
            let (additional_cached, to_download, additional_resolved) = self
                .resolve_uncached_fast(&packages_to_resolve, cache_manager, debug)
                .await?;

            cached_packages.extend(additional_cached);
            packages_to_download.extend(to_download);
            all_resolved.extend(additional_resolved);

            if debug {
                pacm_logger::debug(
                    &format!("Fast resolution completed in {:?}", resolve_start.elapsed()),
                    debug,
                );
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Total analysis completed in {:?} - {} cached, {} to download",
                    start_time.elapsed(),
                    cached_packages.len(),
                    packages_to_download.len()
                ),
                debug,
            );
        }

        Ok((
            cached_packages,
            packages_to_download,
            direct_names,
            all_resolved,
        ))
    }

    async fn resolve_all_parallel(
        &self,
        direct_deps: &[(String, String)],
        _use_lockfile: bool,
        debug: bool,
    ) -> Result<(HashSet<String>, HashMap<String, ResolvedPackage>)> {
        let mut direct_package_names = HashSet::with_capacity(direct_deps.len());
        for (name, _) in direct_deps {
            direct_package_names.insert(name.clone());
        }

        let client = self.client.clone();
        let resolution_cache = self.resolution_cache.clone();

        let resolve_tasks: Vec<_> = direct_deps
            .iter()
            .map(|(name, version_or_range)| {
                let client = client.clone();
                let resolution_cache = resolution_cache.clone();
                let name = name.clone();
                let version_or_range = version_or_range.clone();

                async move {
                    if !debug {
                        pacm_logger::status(&format!(
                            "Analyzing package requirements for {}...",
                            name
                        ));
                    }

                    let cache_key = format!("{}@{}", name, version_or_range);

                    {
                        let cache = resolution_cache.lock().await;
                        if let Some(cached_result) = cache.get(&cache_key) {
                            return Ok(cached_result.clone());
                        }
                    }

                    let mut seen = HashSet::with_capacity(100);
                    let result =
                        resolve_full_tree_async(client, &name, &version_or_range, &mut seen)
                            .await
                            .map_err(|e| {
                                PackageManagerError::VersionResolutionFailed(
                                    name.clone(),
                                    format!("Failed to resolve {}: {}", name, e),
                                )
                            });

                    if let Ok(ref packages) = result {
                        let mut cache = resolution_cache.lock().await;
                        cache.insert(cache_key, packages.clone());
                    }

                    result
                }
            })
            .collect();

        let resolve_results = join_all(resolve_tasks).await;

        let mut all_resolved_packages = Vec::with_capacity(resolve_results.len() * 10);
        for (i, result) in resolve_results.into_iter().enumerate() {
            match result {
                Ok(resolved_tree) => {
                    if debug {
                        pacm_logger::debug(
                            &format!(
                                "Resolved dependency tree {} with {} packages",
                                i + 1,
                                resolved_tree.len()
                            ),
                            debug,
                        );
                    }
                    all_resolved_packages.extend(resolved_tree)
                }
                Err(e) => {
                    pacm_logger::error(&format!(
                        "Failed to resolve dependency {}: {}",
                        direct_deps[i].0, e
                    ));
                    return Err(e);
                }
            }
        }

        let mut unique_packages = HashMap::with_capacity(all_resolved_packages.len());
        for pkg in all_resolved_packages {
            let key = format!("{}@{}", pkg.name, pkg.version);
            unique_packages.insert(key, pkg);
        }

        if debug {
            pacm_logger::debug(
                &format!("Resolved {} unique packages total", unique_packages.len()),
                debug,
            );
        }

        Ok((direct_package_names, unique_packages))
    }

    async fn separate_cached_fast(
        &self,
        resolved_packages: &HashMap<String, ResolvedPackage>,
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(Vec<CachedPackage>, Vec<ResolvedPackage>)> {
        let mut cached_packages = Vec::with_capacity(resolved_packages.len());
        let mut packages_to_download = Vec::with_capacity(resolved_packages.len());

        let cache_lookup_tasks: Vec<_> = resolved_packages
            .iter()
            .map(|(key, pkg)| {
                let key = key.clone();
                let pkg = pkg.clone();
                async move {
                    if let Some(cached) = cache_manager.get(&key).await {
                        (key, Some(cached), pkg)
                    } else {
                        (key, None, pkg)
                    }
                }
            })
            .collect();

        let cache_results = join_all(cache_lookup_tasks).await;

        for (key, cached_opt, pkg) in cache_results {
            if let Some(cached) = cached_opt {
                cached_packages.push(cached);
                if debug {
                    pacm_logger::debug(&format!("Cache hit: {}", key), debug);
                }
            } else {
                packages_to_download.push(pkg);
            }
        }

        Ok((cached_packages, packages_to_download))
    }

    async fn resolve_uncached_fast(
        &self,
        packages_to_resolve: &[(String, String)],
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        if packages_to_resolve.is_empty() {
            return Ok((Vec::new(), Vec::new(), HashMap::new()));
        }

        let (_, all_resolved) = self
            .resolve_all_parallel(packages_to_resolve, false, debug)
            .await?;

        let (cached_packages, packages_to_download) = self
            .separate_cached_fast(&all_resolved, cache_manager, debug)
            .await?;

        Ok((cached_packages, packages_to_download, all_resolved))
    }

    pub async fn resolve_deps(
        &self,
        direct_deps: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashSet<String>,
        HashMap<String, ResolvedPackage>,
    )> {
        let cache_manager = CacheManager::new();
        cache_manager.build_index(debug).await?;

        self.resolve_deps_optimized(direct_deps, use_lockfile, &cache_manager, debug)
            .await
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}
