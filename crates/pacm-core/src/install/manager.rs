use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use super::cache::CacheManager;
use super::resolver::DependencyResolver;
use super::types::CachedPackage;
use crate::download::PackageDownloader;
use crate::linker::PackageLinker;
use pacm_error::{PackageManagerError, Result};
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::{DependencyType, read_package_json};
use pacm_resolver::ResolvedPackage;

pub struct InstallManager {
    downloader: PackageDownloader,
    linker: PackageLinker,
    cache: CacheManager,
    resolver: DependencyResolver,
}

impl InstallManager {
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

        let (deps, use_lockfile) = self.load_deps(&path)?;

        if deps.is_empty() {
            pacm_logger::finish("No dependencies to install");
            return Ok(());
        }

        let cache_start = std::time::Instant::now();
        self.cache.build_index(debug).await?;

        if debug {
            pacm_logger::debug(
                &format!("Cache index built in {:?}", cache_start.elapsed()),
                debug,
            );
        }

        if let Some(cached_result) = self
            .check_all_cached_optimized(&deps, use_lockfile, debug)
            .await?
        {
            let total_time = start_time.elapsed();
            pacm_logger::debug(
                &format!(
                    "All packages cached - completed installation in {:?}",
                    total_time
                ),
                debug,
            );

            return self
                .install_from_cache_only(cached_result, &path, debug)
                .await;
        }

        self.install_mixed_optimized(&deps, use_lockfile, &path, debug)
            .await
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

    async fn check_all_cached_optimized(
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
            return Ok(None); // Early exit - not all cached
        }

        pacm_logger::status("All direct dependencies found in cache - checking full tree...");

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
                // Not all packages are cached
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

    async fn install_mixed_optimized(
        &self,
        deps: &[(String, String)],
        use_lockfile: bool,
        path: &PathBuf,
        debug: bool,
    ) -> Result<()> {
        let resolution_start = std::time::Instant::now();

        let (cached_packages, packages_to_download, direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(deps, use_lockfile, &self.cache, debug)
            .await?;

        if debug {
            pacm_logger::debug(
                &format!(
                    "Resolution and cache separation completed in {:?}",
                    resolution_start.elapsed()
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
            let downloaded = self.download_packages(&packages_to_download, debug).await?;
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

        let msg = self.build_finish_message(&cached_packages, &packages_to_download);
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn install_from_cache_only(
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
        self.linker
            .verify_and_fix_cached_package_dependencies(cached, stored, debug)
    }

    fn link_to_project(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_names: &HashSet<String>,
        debug: bool,
    ) -> Result<()> {
        self.linker
            .link_direct_dependencies_to_project(path, stored, direct_names, debug)
    }

    fn update_lock(
        &self,
        path: &PathBuf,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_names: &HashSet<String>,
    ) -> Result<()> {
        let lock_path = path.join("pacm.lock");
        self.linker
            .update_lockfile_direct_only(&lock_path, stored, direct_names)
    }

    async fn download_packages(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<HashMap<String, (ResolvedPackage, PathBuf)>> {
        pacm_logger::status(&format!("Downloading {} packages...", packages.len()));
        self.downloader.download_parallel(packages, debug).await
    }

    fn link_store_deps(
        &self,
        stored: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        self.linker.link_dependencies_to_store(stored, debug)
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

        self.run_postinstall_scripts(&new_packages, debug)
    }

    fn build_finish_message(
        &self,
        cached: &[CachedPackage],
        downloaded: &[ResolvedPackage],
    ) -> String {
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

    pub fn install_single(
        &self,
        project_dir: &str,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.install_single_async(
            project_dir,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        ))
    }

    pub fn install_single_dependency(
        &self,
        project_dir: &str,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        self.install_single(
            project_dir,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        )
    }

    async fn install_single_async(
        &self,
        project_dir: &str,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        _force: bool,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status(&format!("Installing {}@{}", name, version_range));

        let path = PathBuf::from(project_dir);

        if debug {
            pacm_logger::debug("Building cache index...", debug);
        }
        self.cache.build_index(debug).await?;

        let deps = vec![(name.to_string(), version_range.to_string())];

        if debug {
            pacm_logger::debug(
                &format!(
                    "Starting dependency resolution for {}@{}",
                    name, version_range
                ),
                debug,
            );
        }

        let resolution_timeout = tokio::time::timeout(
            std::time::Duration::from_secs(120), // 2 minute timeout
            self.resolver
                .resolve_deps_optimized(&deps, false, &self.cache, debug),
        )
        .await;

        let (cached_packages, packages_to_download, mut direct_names, resolved_map) =
            match resolution_timeout {
                Ok(result) => result?,
                Err(_) => {
                    return Err(PackageManagerError::NetworkError(format!(
                        "Dependency resolution timed out for {}@{}",
                        name, version_range
                    )));
                }
            };

        if debug {
            pacm_logger::debug(
                &format!(
                    "Resolution completed: {} cached, {} to download",
                    cached_packages.len(),
                    packages_to_download.len()
                ),
                debug,
            );
        }

        direct_names.clear();
        direct_names.insert(name.to_string());

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if packages_to_download.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("All {} packages found in cache", cached_packages.len()),
                    debug,
                );
            }

            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
            self.link_to_project(&path, &stored_packages, &direct_names, debug)?;

            if !no_save {
                self.update_package_json(
                    &path,
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    &stored_packages,
                )?;
            }

            self.update_lock(&path, &stored_packages, &direct_names)?;

            let msg = if cached_packages.len() == 1 {
                format!("{} linked from cache", name)
            } else if cached_packages.len() > 1 {
                format!(
                    "{} and {} dependencies linked from cache",
                    name,
                    cached_packages.len() - 1
                )
            } else {
                format!("{} package resolved", name)
            };
            pacm_logger::finish(&msg);
            return Ok(());
        }

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !packages_to_download.is_empty() {
            let downloaded = self.download_packages(&packages_to_download, debug).await?;
            stored_packages.extend(downloaded);
            self.link_store_deps(&stored_packages, debug)?;
            self.run_post_install(&stored_packages, &packages_to_download, debug)?;
        }

        self.link_to_project(&path, &stored_packages, &direct_names, debug)?;

        if !no_save {
            self.update_package_json(
                &path,
                name,
                version_range,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(&path, &stored_packages, &direct_names)?;

        let msg = self.build_single_finish_message(&cached_packages, &packages_to_download, name);
        pacm_logger::finish(&msg);

        Ok(())
    }

    fn update_package_json(
        &self,
        path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        use pacm_project::{read_package_json, write_package_json};

        let mut pkg = read_package_json(path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let actual_version = stored_packages
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{}@", name)))
            .map(|(_, (resolved_pkg, _))| resolved_pkg.version.as_str())
            .unwrap_or(version_range);

        pkg.add_dependency(name, actual_version, dep_type, save_exact);

        write_package_json(path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        Ok(())
    }

    fn build_single_finish_message(
        &self,
        cached: &[CachedPackage],
        downloaded: &[ResolvedPackage],
        package_name: &str,
    ) -> String {
        let cached_count = cached.len();
        let downloaded_count = downloaded.len();
        let total_count = cached_count + downloaded_count;

        if total_count == 1 {
            if cached_count == 1 {
                format!("{} linked from cache", package_name)
            } else {
                format!("{} downloaded and installed", package_name)
            }
        } else {
            if cached_count > 0 && downloaded_count > 0 {
                format!(
                    "{} and {} dependencies installed ({} from cache, {} downloaded)",
                    package_name,
                    total_count - 1,
                    cached_count,
                    downloaded_count
                )
            } else if cached_count > 0 {
                format!(
                    "{} and {} dependencies linked from cache",
                    package_name,
                    cached_count - 1
                )
            } else {
                format!(
                    "{} and {} dependencies downloaded and installed",
                    package_name,
                    downloaded_count - 1
                )
            }
        }
    }

    fn run_postinstall_scripts(
        &self,
        packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        for (_package_key, (resolved_pkg, store_path)) in packages {
            let package_json_path = store_path.join("package").join("package.json");

            if package_json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                    if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(scripts) = pkg_json.get("scripts") {
                            if let Some(postinstall) = scripts.get("postinstall") {
                                if let Some(script_cmd) = postinstall.as_str() {
                                    pacm_logger::debug(
                                        &format!(
                                            "Running postinstall for {}: {}",
                                            resolved_pkg.name, script_cmd
                                        ),
                                        debug,
                                    );

                                    let package_dir = store_path.join("package");
                                    let status = if cfg!(target_os = "windows") {
                                        Command::new("cmd")
                                            .args(["/C", script_cmd])
                                            .current_dir(&package_dir)
                                            .status()
                                    } else {
                                        Command::new("sh")
                                            .arg("-c")
                                            .arg(script_cmd)
                                            .current_dir(&package_dir)
                                            .status()
                                    };

                                    match status {
                                        Ok(exit_status) if exit_status.success() => {
                                            pacm_logger::debug(
                                                &format!(
                                                    "Postinstall script completed for {}",
                                                    resolved_pkg.name
                                                ),
                                                debug,
                                            );
                                        }
                                        Ok(_) => {
                                            pacm_logger::debug(
                                                &format!(
                                                    "Postinstall script failed for {}",
                                                    resolved_pkg.name
                                                ),
                                                debug,
                                            );
                                        }
                                        Err(e) => {
                                            pacm_logger::debug(
                                                &format!(
                                                    "Failed to run postinstall for {}: {}",
                                                    resolved_pkg.name, e
                                                ),
                                                debug,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for InstallManager {
    fn default() -> Self {
        Self::new()
    }
}
