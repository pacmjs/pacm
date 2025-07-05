use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::cache::CacheManager;
use super::types::CachedPackage;
use pacm_constants::USER_AGENT;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_registry;
use pacm_resolver::{ResolvedPackage, resolve_full_tree_async};
use pacm_symcap::SystemCapabilities;

pub struct DependencyResolver {
    client: Arc<reqwest::Client>,
    resolution_cache: Arc<Mutex<HashMap<String, Vec<ResolvedPackage>>>>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        let system_caps = SystemCapabilities::get();
        let pool_size = system_caps.optimal_parallel_downloads;

        Self {
            client: Arc::new(
                reqwest::Client::builder()
                    .pool_max_idle_per_host(pool_size)
                    .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
                    .timeout(std::time::Duration::from_secs(30)) // Reduced from 45s
                    .connect_timeout(std::time::Duration::from_secs(10)) // Reduced from 20s
                    .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
                    .tcp_nodelay(true)
                    .user_agent(USER_AGENT)
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new()),
            ),
            resolution_cache: Arc::new(Mutex::new(HashMap::with_capacity(2000))), // Increased capacity
        }
    }

    pub fn get_client(&self) -> Arc<reqwest::Client> {
        self.client.clone()
    }

    fn read_dependencies_from_cached_package(
        cached_package: &CachedPackage,
        debug: bool,
    ) -> (HashMap<String, String>, HashMap<String, String>) {
        let package_json_path = cached_package
            .store_path
            .join("package")
            .join("package.json");

        if !package_json_path.exists() {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "No package.json found for cached package {}",
                        cached_package.name
                    ),
                    debug,
                );
            }
            return (HashMap::new(), HashMap::new());
        }

        match std::fs::read_to_string(&package_json_path) {
            Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(pkg_data) => {
                    let dependencies: HashMap<String, String> = pkg_data
                        .get("dependencies")
                        .and_then(|d| d.as_object())
                        .map(|deps| {
                            deps.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let optional_dependencies: HashMap<String, String> = pkg_data
                        .get("optionalDependencies")
                        .and_then(|d| d.as_object())
                        .map(|deps| {
                            deps.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    if debug && (!dependencies.is_empty() || !optional_dependencies.is_empty()) {
                        pacm_logger::debug(
                            &format!(
                                "Read {} dependencies and {} optional dependencies from {}",
                                dependencies.len(),
                                optional_dependencies.len(),
                                cached_package.name
                            ),
                            debug,
                        );
                    }

                    (dependencies, optional_dependencies)
                }
                Err(e) => {
                    if debug {
                        pacm_logger::debug(
                            &format!(
                                "Failed to parse package.json for {}: {}",
                                cached_package.name, e
                            ),
                            debug,
                        );
                    }
                    (HashMap::new(), HashMap::new())
                }
            },
            Err(e) => {
                if debug {
                    pacm_logger::debug(
                        &format!(
                            "Failed to read package.json for {}: {}",
                            cached_package.name, e
                        ),
                        debug,
                    );
                }
                (HashMap::new(), HashMap::new())
            }
        }
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

                let (dependencies, optional_dependencies) =
                    Self::read_dependencies_from_cached_package(&cached, debug);

                let resolved_pkg = ResolvedPackage {
                    name: cached.name.clone(),
                    version: cached.version.clone(),
                    resolved: cached.resolved.clone(),
                    integrity: cached.integrity.clone(),
                    dependencies,
                    optional_dependencies,
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

    pub async fn resolve_all_parallel(
        &self,
        direct_deps: &[(String, String)],
        _use_lockfile: bool,
        debug: bool,
    ) -> Result<(HashSet<String>, HashMap<String, ResolvedPackage>)> {
        let system_caps = SystemCapabilities::get();
        let mut direct_package_names = HashSet::with_capacity(direct_deps.len());
        for (name, _) in direct_deps {
            direct_package_names.insert(name.clone());
        }

        let batch_size = system_caps.get_network_batch_size(direct_deps.len());
        let batches: Vec<_> = direct_deps.chunks(batch_size).collect();

        if debug {
            pacm_logger::debug(
                &format!(
                    "Resolving {} packages in {} batches of up to {} packages each",
                    direct_deps.len(),
                    batches.len(),
                    batch_size
                ),
                debug,
            );
        }

        let client = self.client.clone();
        let resolution_cache = self.resolution_cache.clone();

        let mut all_resolved_packages = Vec::with_capacity(direct_deps.len() * 8);

        for (batch_idx, batch) in batches.into_iter().enumerate() {
            if debug && batch.len() > 1 {
                pacm_logger::debug(
                    &format!(
                        "Processing batch {} with {} packages",
                        batch_idx + 1,
                        batch.len()
                    ),
                    debug,
                );
            }

            let resolve_tasks: Vec<_> = batch
                .iter()
                .map(|(name, version_or_range)| {
                    let client = client.clone();
                    let resolution_cache = resolution_cache.clone();
                    let name = name.clone();
                    let version_or_range = version_or_range.clone();

                    async move {
                        let cache_key = format!("{}@{}", name, version_or_range);

                        {
                            let cache = resolution_cache.lock().await;
                            if let Some(cached_result) = cache.get(&cache_key) {
                                return Ok(cached_result.clone());
                            }
                        }

                        if system_caps.should_skip_transitive_analysis(&name) {
                            if let Ok(pkg_data) =
                                pacm_registry::fetch_package_info_async(client.clone(), &name).await
                            {
                                if let Some(latest_version) = pkg_data.dist_tags.get("latest") {
                                    let simple_pkg = ResolvedPackage {
                                        name: name.clone(),
                                        version: latest_version.clone(),
                                        resolved: format!(
                                            "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                            name, name, latest_version
                                        ),
                                        integrity: String::new(),
                                        dependencies: HashMap::new(), // Skip dependency resolution for simple packages
                                        optional_dependencies: HashMap::new(),
                                        os: None,
                                        cpu: None,
                                    };

                                    let result = vec![simple_pkg];
                                    let mut cache = resolution_cache.lock().await;
                                    cache.insert(cache_key, result.clone());
                                    return Ok(result);
                                }
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

            for (i, result) in resolve_results.into_iter().enumerate() {
                match result {
                    Ok(resolved_tree) => {
                        if debug && resolved_tree.len() > 5 {
                            pacm_logger::debug(
                                &format!(
                                    "Resolved {} with {} packages",
                                    batch[i].0,
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
                            batch[i].0, e
                        ));
                        return Err(e);
                    }
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

    pub async fn separate_cached_fast(
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

    pub async fn resolve_deps_fast(
        &self,
        direct_deps: &[(String, String)],
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashSet<String>,
        HashMap<String, ResolvedPackage>,
    )> {
        let system_caps = SystemCapabilities::get();
        let start_time = std::time::Instant::now();

        if debug {
            pacm_logger::debug(
                &format!(
                    "Starting fast resolution for {} packages (using {} parallel ops)",
                    direct_deps.len(),
                    system_caps.optimal_parallel_resolutions
                ),
                debug,
            );
        }

        let cache_check_start = std::time::Instant::now();
        let direct_cache_results = cache_manager.get_batch_direct(direct_deps).await;

        if debug {
            pacm_logger::debug(
                &format!("Cache check completed in {:?}", cache_check_start.elapsed()),
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
                    pacm_logger::debug(&format!("Cache hit: {}", name), debug);
                }
                cached_packages.push(cached.clone());

                let resolved_pkg = ResolvedPackage {
                    name: cached.name.clone(),
                    version: cached.version.clone(),
                    resolved: cached.resolved.clone(),
                    integrity: cached.integrity.clone(),
                    dependencies: HashMap::new(), // Will be filled if needed
                    optional_dependencies: HashMap::new(),
                    os: None,
                    cpu: None,
                };

                let key = format!("{}@{}", cached.name, cached.version);
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

            let batch_size = system_caps.get_optimal_batch_size(packages_to_resolve.len());
            let batches: Vec<_> = packages_to_resolve.chunks(batch_size).collect();

            for batch in batches {
                let (additional_cached, to_download, additional_resolved) = self
                    .resolve_batch_optimized(batch, cache_manager, debug)
                    .await?;

                cached_packages.extend(additional_cached);
                packages_to_download.extend(to_download);
                all_resolved.extend(additional_resolved);
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Fast resolution completed in {:?} - {} cached, {} to download",
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

    async fn resolve_batch_optimized(
        &self,
        packages: &[(String, String)],
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        let system_caps = SystemCapabilities::get();

        if packages.len() <= 2 || !system_caps.should_use_parallel_for_count(packages.len()) {
            return self
                .resolve_sequential(packages, cache_manager, debug)
                .await;
        }

        let client = self.client.clone();
        let resolution_cache = self.resolution_cache.clone();

        let resolve_tasks: Vec<_> = packages
            .iter()
            .map(|(name, version_range)| {
                let client = client.clone();
                let resolution_cache = resolution_cache.clone();
                let name = name.clone();
                let version_range = version_range.clone();

                async move {
                    let cache_key = format!("{}@{}", name, version_range);

                    {
                        let cache = resolution_cache.lock().await;
                        if let Some(cached_result) = cache.get(&cache_key) {
                            return Ok((name, cached_result.clone()));
                        }
                    }

                    let mut seen = HashSet::with_capacity(50);
                    let result = resolve_full_tree_async(client, &name, &version_range, &mut seen)
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

                    result.map(|packages| (name, packages))
                }
            })
            .collect();

        let resolve_results = join_all(resolve_tasks).await;

        let mut all_resolved_packages = Vec::new();
        for result in resolve_results {
            match result {
                Ok((name, resolved_tree)) => {
                    if debug {
                        pacm_logger::debug(
                            &format!("Resolved {} with {} packages", name, resolved_tree.len()),
                            debug,
                        );
                    }
                    all_resolved_packages.extend(resolved_tree);
                }
                Err(e) => {
                    pacm_logger::error(&format!("Failed to resolve dependency: {}", e));
                    return Err(e);
                }
            }
        }

        let mut unique_packages = HashMap::with_capacity(all_resolved_packages.len());
        for pkg in all_resolved_packages {
            let key = format!("{}@{}", pkg.name, pkg.version);
            unique_packages.insert(key, pkg);
        }

        let (cached_packages, packages_to_download) = self
            .separate_cached_fast(&unique_packages, cache_manager, debug)
            .await?;

        Ok((cached_packages, packages_to_download, unique_packages))
    }

    async fn resolve_sequential(
        &self,
        packages: &[(String, String)],
        cache_manager: &CacheManager,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        let mut all_resolved = HashMap::new();

        for (name, version_range) in packages {
            if !debug {
                pacm_logger::status(&format!("Analyzing {}...", name));
            }

            let cache_key = format!("{}@{}", name, version_range);

            {
                let cache = self.resolution_cache.lock().await;
                if let Some(cached_result) = cache.get(&cache_key) {
                    for pkg in cached_result {
                        let key = format!("{}@{}", pkg.name, pkg.version);
                        all_resolved.insert(key, pkg.clone());
                    }
                    continue;
                }
            }

            let mut seen = HashSet::with_capacity(50);
            match resolve_full_tree_async(self.client.clone(), name, version_range, &mut seen).await
            {
                Ok(resolved_tree) => {
                    {
                        let mut cache = self.resolution_cache.lock().await;
                        cache.insert(cache_key, resolved_tree.clone());
                    }

                    for pkg in resolved_tree {
                        let key = format!("{}@{}", pkg.name, pkg.version);
                        all_resolved.insert(key, pkg);
                    }
                }
                Err(e) => {
                    return Err(PackageManagerError::VersionResolutionFailed(
                        name.clone(),
                        format!("Failed to resolve {}: {}", name, e),
                    ));
                }
            }
        }

        let (cached_packages, packages_to_download) = self
            .separate_cached_fast(&all_resolved, cache_manager, debug)
            .await?;

        Ok((cached_packages, packages_to_download, all_resolved))
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}
