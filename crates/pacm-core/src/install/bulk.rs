use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::cache::CacheManager;
use super::resolver::DependencyResolver;
use super::smart_analyzer::{PackageComplexity, SmartDependencyAnalyzer};
use super::types::CachedPackage;
use crate::download::PackageDownloader;
use crate::linker::PackageLinker;
use pacm_error::{PackageManagerError, Result};
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::read_package_json;
use pacm_resolver::{ResolvedPackage, is_platform_compatible};

pub struct BulkInstaller {
    downloader: PackageDownloader,
    linker: PackageLinker,
    cache: CacheManager,
    resolver: DependencyResolver,
    smart_analyzer: SmartDependencyAnalyzer,
}

impl BulkInstaller {
    pub fn new() -> Self {
        let cache = CacheManager::new();
        let smart_analyzer = SmartDependencyAnalyzer::new(cache.clone());

        Self {
            downloader: PackageDownloader::new(),
            linker: PackageLinker {},
            cache,
            resolver: DependencyResolver::new(),
            smart_analyzer,
        }
    }

    pub fn install_all(&self, project_dir: &str, debug: bool) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.install_all_async(project_dir, debug))
    }

    async fn install_all_async(&self, project_dir: &str, debug: bool) -> Result<()> {
        let start_time = std::time::Instant::now();
        let path = PathBuf::from(project_dir);
        let _pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let (all_deps, use_lockfile) = self.load_deps(&path)?;

        if all_deps.is_empty() {
            pacm_logger::finish("No dependencies to install");
            return Ok(());
        }

        let deps = self.check_existing_pkgs(&path, &all_deps, use_lockfile, debug)?;

        if deps.is_empty() {
            pacm_logger::finish("All dependencies are already installed");
            return Ok(());
        }

        self.cache.build_index(debug).await?;

        if let Some(cached_result) = self.check_all_cached(&deps, use_lockfile, debug).await? {
            let total_time = start_time.elapsed();
            pacm_logger::debug(
                &format!(
                    "All packages cached - completed installation in {:?}",
                    total_time
                ),
                debug,
            );

            let direct_count = if use_lockfile {
                self.get_actual_direct_dependencies(&path)?.len()
            } else {
                all_deps.len()
            };

            return self
                .install_cached_only(cached_result, &path, use_lockfile, direct_count, debug)
                .await;
        }

        let analysis_start = std::time::Instant::now();

        if !debug {
            pacm_logger::status(&format!("Analyzing {} dependencies...", deps.len()));
        }

        let package_analyses = self.smart_analyzer.analyze_packages(&deps, debug).await?;

        if debug {
            pacm_logger::debug(
                &format!("Smart analysis completed in {:?}", analysis_start.elapsed()),
                debug,
            );
        }

        let mut trivial_packages = Vec::new();
        let mut simple_packages = Vec::new();
        let mut moderate_packages = Vec::new();
        let mut complex_packages = Vec::new();

        for (i, analysis) in package_analyses.iter().enumerate() {
            let (name, version) = &deps[i];
            match analysis.complexity {
                PackageComplexity::Trivial => {
                    trivial_packages.push((name.clone(), version.clone()))
                }
                PackageComplexity::Simple => simple_packages.push((name.clone(), version.clone())),
                PackageComplexity::Moderate => {
                    moderate_packages.push((name.clone(), version.clone()))
                }
                PackageComplexity::Complex => {
                    complex_packages.push((name.clone(), version.clone()))
                }
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Package complexity breakdown: {} trivial, {} simple, {} moderate, {} complex",
                    trivial_packages.len(),
                    simple_packages.len(),
                    moderate_packages.len(),
                    complex_packages.len()
                ),
                debug,
            );
        }

        let direct_count = if use_lockfile {
            self.get_actual_direct_dependencies(&path)?.len()
        } else {
            all_deps.len()
        };

        self.install_by_complexity(
            trivial_packages,
            simple_packages,
            moderate_packages,
            complex_packages,
            use_lockfile,
            &path,
            direct_count,
            debug,
        )
        .await
    }

    fn load_deps(&self, path: &PathBuf) -> Result<(Vec<(String, String)>, bool)> {
        let lock_path = path.join("pacm.lock");

        if lock_path.exists() {
            pacm_logger::status("Using existing lockfile...");
            let lockfile = PacmLock::load(&lock_path)
                .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

            let mut deps = Vec::new();

            if !lockfile.packages.is_empty() {
                for (name, lock_package) in &lockfile.packages {
                    deps.push((name.clone(), lock_package.version.clone()));
                }
            } else {
                if let Some(workspace_info) = lockfile.workspaces.get("") {
                    for (name, version) in &workspace_info.dependencies {
                        deps.push((name.clone(), version.clone()));
                    }
                    for (name, version) in &workspace_info.dev_dependencies {
                        deps.push((name.clone(), version.clone()));
                    }
                    for (name, version) in &workspace_info.peer_dependencies {
                        deps.push((name.clone(), version.clone()));
                    }
                    for (name, version) in &workspace_info.optional_dependencies {
                        deps.push((name.clone(), version.clone()));
                    }
                }

                if deps.is_empty() {
                    deps = lockfile
                        .dependencies
                        .iter()
                        .map(|(name, lock_dep)| (name.clone(), lock_dep.version.clone()))
                        .collect();
                }
            }

            Ok((deps, true))
        } else {
            pacm_logger::status("Using package.json dependencies...");
            let pkg = read_package_json(path)
                .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;
            let all_deps = pkg.get_all_dependencies();
            let deps: Vec<(String, String)> = all_deps.into_iter().collect();
            Ok((deps, false))
        }
    }

    async fn check_all_cached(
        &self,
        deps: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<
        Option<(
            Vec<CachedPackage>,
            HashSet<String>,
            HashMap<String, ResolvedPackage>,
        )>,
    > {
        if !self.cache.are_all_cached(deps).await {
            return Ok(None);
        }

        pacm_logger::status("Checking cache for instant installation...");

        let (direct_names, resolved_map) = if use_lockfile {
            let (_, _, direct_names, resolved_map) = self
                .resolver
                .resolve_deps_optimized(deps, use_lockfile, &self.cache, debug)
                .await?;
            (direct_names, resolved_map)
        } else {
            self.resolver
                .resolve_all_parallel(deps, use_lockfile, debug)
                .await?
        };

        let cache_keys: Vec<String> = resolved_map.keys().cloned().collect();
        let batch_results = self.cache.get_batch(&cache_keys).await;

        let mut all_cached_packages = Vec::new();
        for (_key, cached_opt) in batch_results {
            if let Some(cached) = cached_opt {
                all_cached_packages.push(cached);
            } else {
                return Ok(None);
            }
        }

        if all_cached_packages.len() == resolved_map.len() {
            pacm_logger::status(&format!(
                "All {} packages found in cache - installing instantly!",
                all_cached_packages.len()
            ));
            Ok(Some((all_cached_packages, direct_names, resolved_map)))
        } else {
            Ok(None)
        }
    }

    async fn install_cached_only(
        &self,
        (cached_packages, direct_names, resolved_map): (
            Vec<CachedPackage>,
            HashSet<String>,
            HashMap<String, ResolvedPackage>,
        ),
        path: &PathBuf,
        use_lockfile: bool,
        direct_count: usize,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status(&format!(
            "All {} packages found in cache",
            cached_packages.len()
        ));

        let stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        self.link_all_to_project(path, &stored_packages, debug)?;

        super::utils::InstallUtils::run_postinstall_in_project(path, &stored_packages, debug)?;

        self.update_lock(path, &stored_packages, &direct_names, use_lockfile)?;

        let total_count = cached_packages.len();
        let transitive_count = total_count.saturating_sub(direct_count);

        let finish_msg = if transitive_count > 0 {
            format!(
                "{} packages ({} with {} dependencies) linked from cache",
                total_count, direct_count, transitive_count
            )
        } else {
            format!("{} packages linked from cache", total_count)
        };

        pacm_logger::finish(&finish_msg);
        Ok(())
    }

    async fn install_by_complexity(
        &self,
        trivial_packages: Vec<(String, String)>,
        simple_packages: Vec<(String, String)>,
        moderate_packages: Vec<(String, String)>,
        complex_packages: Vec<(String, String)>,
        use_lockfile: bool,
        path: &PathBuf,
        direct_count: usize,
        debug: bool,
    ) -> Result<()> {
        let mut all_cached = Vec::new();
        let mut all_downloaded = Vec::new();
        let mut all_resolved = HashMap::new();

        if !trivial_packages.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("Processing {} trivial packages", trivial_packages.len()),
                    debug,
                );
            }

            let (cached, downloaded, resolved) = self
                .process_trivial_packages(&trivial_packages, debug)
                .await?;
            all_cached.extend(cached);
            all_downloaded.extend(downloaded);
            all_resolved.extend(resolved);
        }

        if !simple_packages.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("Processing {} simple packages", simple_packages.len()),
                    debug,
                );
            }

            let (cached, downloaded, resolved) = self
                .process_simple_packages(&simple_packages, debug)
                .await?;
            all_cached.extend(cached);
            all_downloaded.extend(downloaded);
            all_resolved.extend(resolved);
        }

        if !moderate_packages.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("Processing {} moderate packages", moderate_packages.len()),
                    debug,
                );
            }

            let (cached, downloaded, resolved) = self
                .process_moderate_packages(&moderate_packages, debug)
                .await?;
            all_cached.extend(cached);
            all_downloaded.extend(downloaded);
            all_resolved.extend(resolved);
        }

        if !complex_packages.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("Processing {} complex packages", complex_packages.len()),
                    debug,
                );
            }

            let (cached, downloaded, resolved) = self
                .process_complex_packages(&complex_packages, use_lockfile, debug)
                .await?;
            all_cached.extend(cached);
            all_downloaded.extend(downloaded);
            all_resolved.extend(resolved);
        }

        let compatible_packages_to_download: Vec<ResolvedPackage> = all_downloaded
            .iter()
            .filter(|pkg| is_platform_compatible(&pkg.os, &pkg.cpu))
            .cloned()
            .collect();

        let mut stored_packages = self.build_stored_map(&all_cached, &all_resolved);

        if !compatible_packages_to_download.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "Downloading {} packages",
                        compatible_packages_to_download.len()
                    ),
                    debug,
                );
            }

            let downloaded = self
                .downloader
                .download_parallel(&compatible_packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);
        }

        if !all_cached.is_empty() {
            self.link_cached_deps(&all_cached, &stored_packages, debug)?;
        }

        self.link_all_to_project(path, &stored_packages, debug)?;

        if !stored_packages.is_empty() {
            super::utils::InstallUtils::run_postinstall_in_project(path, &stored_packages, debug)?;
        }

        let direct_names = self.get_actual_direct_dependencies(path)?;
        self.update_lock(path, &stored_packages, &direct_names, use_lockfile)?;

        let msg =
            self.build_finish_msg(&all_cached, &compatible_packages_to_download, direct_count);
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn process_trivial_packages(
        &self,
        packages: &[(String, String)],
        _debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        let mut cached_packages = Vec::new();
        let mut resolved_map = HashMap::new();

        for (name, version) in packages {
            let cache_key = format!("{}@{}", name, version);
            if let Some(cached) = self.cache.get(&cache_key).await {
                cached_packages.push(cached.clone());

                let resolved_pkg = ResolvedPackage {
                    name: name.clone(),
                    version: version.clone(),
                    resolved: cached.resolved.clone(),
                    integrity: cached.integrity.clone(),
                    dependencies: HashMap::new(), // Trivial = no dependencies
                    optional_dependencies: HashMap::new(),
                    os: None,
                    cpu: None,
                };
                resolved_map.insert(cache_key, resolved_pkg);
            }
        }

        Ok((cached_packages, Vec::new(), resolved_map))
    }

    async fn process_simple_packages(
        &self,
        packages: &[(String, String)],
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        self.resolver
            .resolve_deps_fast(packages, &self.cache, debug)
            .await
            .map(|(cached, downloaded, _, resolved)| (cached, downloaded, resolved))
    }

    async fn process_moderate_packages(
        &self,
        packages: &[(String, String)],
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        self.resolver
            .resolve_deps_optimized(packages, false, &self.cache, debug)
            .await
            .map(|(cached, downloaded, _, resolved)| (cached, downloaded, resolved))
    }

    async fn process_complex_packages(
        &self,
        packages: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashMap<String, ResolvedPackage>,
    )> {
        self.resolver
            .resolve_deps_optimized(packages, use_lockfile, &self.cache, debug)
            .await
            .map(|(cached, downloaded, _, resolved)| (cached, downloaded, resolved))
    }

    fn check_existing_pkgs(
        &self,
        path: &PathBuf,
        deps: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<Vec<(String, String)>> {
        super::utils::InstallUtils::check_existing_pkgs(path, deps, use_lockfile, debug)
    }

    fn build_stored_map(
        &self,
        cached: &[CachedPackage],
        resolved: &HashMap<String, ResolvedPackage>,
    ) -> HashMap<String, (ResolvedPackage, PathBuf)> {
        let mut stored = HashMap::new();

        for cached_pkg in cached {
            let key = format!("{}@{}", cached_pkg.name, cached_pkg.version);
            let resolved_pkg = resolved
                .get(&key)
                .cloned()
                .unwrap_or_else(|| ResolvedPackage {
                    name: cached_pkg.name.clone(),
                    version: cached_pkg.version.clone(),
                    resolved: cached_pkg.resolved.clone(),
                    integrity: cached_pkg.integrity.clone(),
                    dependencies: HashMap::new(),
                    optional_dependencies: HashMap::new(),
                    os: None,
                    cpu: None,
                });
            stored.insert(key, (resolved_pkg, cached_pkg.store_path.clone()));
        }

        stored
    }

    fn link_cached_deps(
        &self,
        cached: &[CachedPackage],
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        self.linker.verify_cached_deps(cached, stored, debug)
    }

    fn link_all_to_project(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        self.linker.link_all_to_project(path, stored, debug)
    }

    fn update_lock(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        _direct_names: &HashSet<String>,
        use_lockfile: bool,
    ) -> Result<()> {
        let lock_path = path.join("pacm.lock");

        if use_lockfile {
            self.linker
                .update_lock_from_lockfile_install(&lock_path, stored)
        } else {
            let actual_direct_names = self.get_actual_direct_dependencies(path)?;
            self.linker
                .update_lock_direct(&lock_path, stored, &actual_direct_names)
        }
    }

    fn get_actual_direct_dependencies(&self, path: &PathBuf) -> Result<HashSet<String>> {
        use pacm_project::read_package_json;

        let pkg = read_package_json(path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let mut direct_deps = HashSet::new();

        if let Some(deps) = &pkg.dependencies {
            for name in deps.keys() {
                direct_deps.insert(name.clone());
            }
        }

        if let Some(dev_deps) = &pkg.dev_dependencies {
            for name in dev_deps.keys() {
                direct_deps.insert(name.clone());
            }
        }

        if let Some(peer_deps) = &pkg.peer_dependencies {
            for name in peer_deps.keys() {
                direct_deps.insert(name.clone());
            }
        }

        if let Some(opt_deps) = &pkg.optional_dependencies {
            for name in opt_deps.keys() {
                direct_deps.insert(name.clone());
            }
        }

        Ok(direct_deps)
    }

    fn build_finish_msg(
        &self,
        cached: &[CachedPackage],
        downloaded: &[ResolvedPackage],
        direct_count: usize,
    ) -> String {
        let cached_count = cached.len();
        let downloaded_count = downloaded.len();
        let total_count = cached_count + downloaded_count;
        let transitive_count = total_count.saturating_sub(direct_count);

        if cached_count > 0 && downloaded_count > 0 {
            if transitive_count > 0 {
                format!(
                    "{} packages installed ({} direct, {} transitive) - {} from cache, {} downloaded",
                    total_count, direct_count, transitive_count, cached_count, downloaded_count
                )
            } else {
                format!(
                    "{} packages installed - {} from cache, {} downloaded",
                    total_count, cached_count, downloaded_count
                )
            }
        } else if cached_count > 0 {
            if transitive_count > 0 {
                format!(
                    "{} packages ({} direct, {} transitive) linked from cache",
                    total_count, direct_count, transitive_count
                )
            } else {
                format!("{} packages linked from cache", total_count)
            }
        } else if downloaded_count > 0 {
            if transitive_count > 0 {
                format!(
                    "{} packages ({} direct, {} transitive) downloaded and installed",
                    total_count, direct_count, transitive_count
                )
            } else {
                format!("{} packages downloaded and installed", total_count)
            }
        } else {
            "No packages installed".to_string()
        }
    }
}

impl Default for BulkInstaller {
    fn default() -> Self {
        Self::new()
    }
}
