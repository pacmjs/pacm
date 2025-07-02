use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::cache::CacheManager;
use super::resolver::DependencyResolver;
use super::types::CachedPackage;
use crate::download::PackageDownloader;
use crate::linker::PackageLinker;
use pacm_error::{PackageManagerError, Result};
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::read_package_json;
use pacm_resolver::ResolvedPackage;

pub struct BulkInstaller {
    downloader: PackageDownloader,
    linker: PackageLinker,
    cache: CacheManager,
    resolver: DependencyResolver,
}

impl BulkInstaller {
    pub fn new() -> Self {
        Self {
            downloader: PackageDownloader::new(),
            linker: PackageLinker {},
            cache: CacheManager::new(),
            resolver: DependencyResolver::new(),
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

            return self.install_cached_only(cached_result, &path, debug).await;
        }

        self.install_mixed(&deps, use_lockfile, &path, debug).await
    }

    fn load_deps(&self, path: &PathBuf) -> Result<(Vec<(String, String)>, bool)> {
        let lock_path = path.join("pacm.lock");

        if lock_path.exists() {
            pacm_logger::status("Using existing lockfile...");
            let lockfile = PacmLock::load(&lock_path)
                .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

            let deps: Vec<(String, String)> = lockfile
                .dependencies
                .iter()
                .map(|(name, lock_dep)| (name.clone(), lock_dep.version.clone()))
                .collect();
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

        let (_, _, direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(deps, use_lockfile, &self.cache, debug)
            .await?;

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

    async fn install_mixed(
        &self,
        deps: &[(String, String)],
        use_lockfile: bool,
        path: &PathBuf,
        debug: bool,
    ) -> Result<()> {
        let analysis_start = std::time::Instant::now();

        let (cached_packages, packages_to_download, direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(deps, use_lockfile, &self.cache, debug)
            .await?;

        if debug {
            pacm_logger::debug(
                &format!(
                    "Package analysis completed in {:?}",
                    analysis_start.elapsed()
                ),
                debug,
            );
        }

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if !cached_packages.is_empty() {
            let cache_link_start = std::time::Instant::now();
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;

            if debug {
                pacm_logger::debug(
                    &format!("Cached packages linked in {:?}", cache_link_start.elapsed()),
                    debug,
                );
            }
        }

        if !packages_to_download.is_empty() {
            let download_start = std::time::Instant::now();
            let downloaded = self
                .downloader
                .download_parallel(&packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);

            if debug {
                pacm_logger::debug(
                    &format!("Downloads completed in {:?}", download_start.elapsed()),
                    debug,
                );
            }

            let link_start = std::time::Instant::now();
            self.link_store_deps(&stored_packages, debug)?;

            if debug {
                pacm_logger::debug(
                    &format!("Store linking completed in {:?}", link_start.elapsed()),
                    debug,
                );
            }

            self.run_post_install(&stored_packages, &packages_to_download, debug)?;
        }

        let final_start = std::time::Instant::now();
        self.link_to_project(path, &stored_packages, &direct_names, debug)?;
        self.update_lock(path, &stored_packages, &direct_names)?;

        if debug {
            pacm_logger::debug(
                &format!("Final linking completed in {:?}", final_start.elapsed()),
                debug,
            );
        }

        let msg = self.build_finish_msg(&cached_packages, &packages_to_download);
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn install_cached_only(
        &self,
        (cached_packages, direct_names, resolved_map): (
            Vec<CachedPackage>,
            HashSet<String>,
            HashMap<String, ResolvedPackage>,
        ),
        path: &PathBuf,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status(&format!(
            "All {} packages found in cache",
            cached_packages.len()
        ));

        let stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        self.link_to_project(path, &stored_packages, &direct_names, debug)?;
        self.update_lock(path, &stored_packages, &direct_names)?;

        pacm_logger::finish(&format!(
            "{} packages linked from cache",
            cached_packages.len()
        ));
        Ok(())
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

    fn link_to_project(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_names: &HashSet<String>,
        debug: bool,
    ) -> Result<()> {
        self.linker
            .link_direct_to_project(path, stored, direct_names, debug)
    }

    fn update_lock(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_names: &HashSet<String>,
    ) -> Result<()> {
        let lock_path = path.join("pacm.lock");
        self.linker
            .update_lock_direct(&lock_path, stored, direct_names)
    }

    fn link_store_deps(
        &self,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        self.linker.link_deps_to_store(stored, debug)
    }

    fn run_post_install(
        &self,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        downloaded: &[ResolvedPackage],
        debug: bool,
    ) -> Result<()> {
        let new_packages: HashMap<String, (ResolvedPackage, PathBuf)> = stored
            .iter()
            .filter(|(key, _)| {
                downloaded
                    .iter()
                    .any(|pkg| key.starts_with(&format!("{}@", pkg.name)))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        self.run_postinstall(&new_packages, debug)
    }

    fn run_postinstall(
        &self,
        packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        super::utils::InstallUtils::run_postinstall(packages, debug)
    }

    fn build_finish_msg(&self, cached: &[CachedPackage], downloaded: &[ResolvedPackage]) -> String {
        let cached_count = cached.len();
        let downloaded_count = downloaded.len();
        let total_count = cached_count + downloaded_count;

        if cached_count > 0 && downloaded_count > 0 {
            format!(
                "{} packages installed ({} from cache, {} downloaded)",
                total_count, cached_count, downloaded_count
            )
        } else if cached_count > 0 {
            format!("{} packages linked from cache", cached_count)
        } else {
            format!("{} packages downloaded and installed", downloaded_count)
        }
    }
}

impl Default for BulkInstaller {
    fn default() -> Self {
        Self::new()
    }
}
