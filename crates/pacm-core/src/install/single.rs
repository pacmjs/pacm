use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::cache::CacheManager;
use super::fast_path::{FastPathAnalyzer, InstallationPath};
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_project::DependencyType;
use pacm_resolver::{ResolvedPackage, is_platform_compatible};

use crate::download::PackageDownloader;
use crate::linker::PackageLinker;

use super::resolver::DependencyResolver;
use super::types::CachedPackage;

pub struct SingleInstaller {
    downloader: PackageDownloader,
    linker: PackageLinker,
    cache: CacheManager,
    resolver: DependencyResolver,
    fast_path_analyzer: FastPathAnalyzer,
}

impl SingleInstaller {
    pub fn new() -> Self {
        let cache = CacheManager::new();
        let fast_path_analyzer = FastPathAnalyzer::new(cache.clone());

        Self {
            downloader: PackageDownloader::new(),
            linker: PackageLinker {},
            cache,
            resolver: DependencyResolver::new(),
            fast_path_analyzer,
        }
    }

    pub fn install(
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

        rt.block_on(self.install_async(
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

    pub fn install_batch(
        &self,
        project_dir: &str,
        packages: &[(String, String)], // (name, version_range) pairs
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.install_batch_async(
            project_dir,
            packages,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        ))
    }

    async fn install_async(
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
        let path = PathBuf::from(project_dir);

        if self.check_existing(
            &path,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            debug,
        )? {
            return Ok(());
        }

        self.cache.build_index(debug).await?;

        let install_path = self
            .fast_path_analyzer
            .analyze_single_package(name, version_range, debug)
            .await?;

        match install_path {
            InstallationPath::InstantLink {
                cached_packages,
                skip_dependency_check: _,
            } => {
                self.install_instant_link(
                    &path,
                    &cached_packages[0],
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await
            }
            InstallationPath::CachedWithDeps {
                main_package,
                need_dep_resolution: _,
            } => {
                self.install_cached_with_minimal_deps(
                    &path,
                    &main_package,
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await
            }
            InstallationPath::OptimizedDownload {
                can_skip_transitive,
                estimated_complexity: _,
            } => {
                if can_skip_transitive {
                    self.install_simple_download(
                        &path,
                        name,
                        version_range,
                        dep_type,
                        save_exact,
                        no_save,
                        debug,
                    )
                    .await
                } else {
                    self.install_optimized_path(
                        &path,
                        name,
                        version_range,
                        dep_type,
                        save_exact,
                        no_save,
                        debug,
                    )
                    .await
                }
            }
            InstallationPath::FullResolution => {
                self.install_full_path(
                    &path,
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await
            }
        }
    }

    async fn install_instant_link(
        &self,
        project_path: &PathBuf,
        cached_package: &CachedPackage,
        name: &str,
        _version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(&format!("Using instant link for {}", name), debug);
        } else {
            pacm_logger::status(&format!("Linking {} from cache...", name));
        }

        let mut stored_packages = HashMap::new();
        let key = format!("{}@{}", cached_package.name, cached_package.version);
        stored_packages.insert(
            key,
            (
                ResolvedPackage {
                    name: cached_package.name.clone(),
                    version: cached_package.version.clone(),
                    resolved: cached_package.resolved.clone(),
                    integrity: cached_package.integrity.clone(),
                    dependencies: HashMap::new(),
                    optional_dependencies: HashMap::new(),
                    os: None,
                    cpu: None,
                },
                cached_package.store_path.clone(),
            ),
        );

        self.link_all_to_project(project_path, &stored_packages, debug)?;

        if !no_save {
            self.update_package_json(
                project_path,
                name,
                &cached_package.version,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        let direct_names = [name.to_string()].iter().cloned().collect();
        self.update_lock(project_path, &stored_packages, &direct_names)?;

        pacm_logger::finish(&format!("{} linked instantly from cache", name));
        Ok(())
    }

    async fn install_cached_with_minimal_deps(
        &self,
        project_path: &PathBuf,
        main_package: &CachedPackage,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(
                &format!("Using cached path with minimal deps for {}", name),
                debug,
            );
        } else {
            pacm_logger::status(&format!("Analyzing minimal requirements for {}...", name));
        }

        let deps = vec![(name.to_string(), version_range.to_string())];

        let (cached_packages, packages_to_download, direct_names, all_resolved_packages) = self
            .resolver
            .resolve_deps_fast(&deps, &self.cache, debug)
            .await?;

        let mut stored_packages = self.build_stored_map(&cached_packages, &all_resolved_packages);

        if !packages_to_download.is_empty() {
            let compatible_packages: Vec<_> = packages_to_download
                .into_iter()
                .filter(|pkg| is_platform_compatible(&pkg.os, &pkg.cpu))
                .collect();

            if !compatible_packages.is_empty() {
                let downloaded = self
                    .downloader
                    .download_parallel(&compatible_packages, debug)
                    .await?;
                stored_packages.extend(downloaded);
            }
        }

        self.link_all_to_project(project_path, &stored_packages, debug)?;

        if !no_save {
            self.update_package_json(
                project_path,
                name,
                &main_package.version,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(project_path, &stored_packages, &direct_names)?;

        let msg = if cached_packages.len() == 1 {
            format!("{} linked from cache", name)
        } else {
            format!(
                "{} with {} dependencies linked",
                name,
                cached_packages.len() - 1
            )
        };
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn install_simple_download(
        &self,
        project_path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(&format!("Using simple download path for {}", name), debug);
        } else {
            pacm_logger::status(&format!("Downloading {}...", name));
        }

        let mut seen = HashSet::new();
        let resolved_packages = pacm_resolver::resolve_full_tree_async(
            self.resolver.get_client(),
            name,
            version_range,
            &mut seen,
        )
        .await
        .map_err(|e| {
            PackageManagerError::VersionResolutionFailed(
                name.to_string(),
                format!("Failed to resolve {}: {}", name, e),
            )
        })?;

        let compatible_packages: Vec<_> = resolved_packages
            .into_iter()
            .filter(|pkg| is_platform_compatible(&pkg.os, &pkg.cpu))
            .collect();

        if compatible_packages.is_empty() {
            return Err(PackageManagerError::NoCompatibleVersions(name.to_string()));
        }

        let downloaded = self
            .downloader
            .download_parallel(&compatible_packages, debug)
            .await?;

        self.link_all_to_project(project_path, &downloaded, debug)?;

        if !no_save {
            let main_package = compatible_packages
                .iter()
                .find(|pkg| pkg.name == name)
                .ok_or_else(|| PackageManagerError::PackageNotFound(name.to_string()))?;

            self.update_package_json(
                project_path,
                name,
                &main_package.version,
                dep_type,
                save_exact,
                &downloaded,
            )?;
        }

        let direct_names = [name.to_string()].iter().cloned().collect();
        self.update_lock(project_path, &downloaded, &direct_names)?;

        let msg = if compatible_packages.len() == 1 {
            format!("{} downloaded and installed", name)
        } else {
            format!(
                "{} with {} dependencies installed",
                name,
                compatible_packages.len() - 1
            )
        };
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn install_optimized_path(
        &self,
        project_path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(&format!("Using optimized path for {}", name), debug);
        } else {
            pacm_logger::status(&format!("Analyzing requirements for {}...", name));
        }

        let deps = vec![(name.to_string(), version_range.to_string())];

        let (cached_packages, packages_to_download, direct_names, all_resolved_packages) = self
            .resolver
            .resolve_deps_fast(&deps, &self.cache, debug)
            .await?;

        let compatible_packages_to_download: Vec<ResolvedPackage> = packages_to_download
            .iter()
            .filter(|pkg| is_platform_compatible(&pkg.os, &pkg.cpu))
            .cloned()
            .collect();

        let mut stored_packages = self.build_stored_map(&cached_packages, &all_resolved_packages);

        if !compatible_packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&compatible_packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);
        }

        self.link_all_to_project(project_path, &stored_packages, debug)?;

        if !no_save {
            let main_package_version = all_resolved_packages
                .values()
                .find(|pkg| pkg.name == name)
                .map(|pkg| &pkg.version)
                .ok_or_else(|| PackageManagerError::PackageNotFound(name.to_string()))?;

            self.update_package_json(
                project_path,
                name,
                main_package_version,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(project_path, &stored_packages, &direct_names)?;

        let msg = self.build_finish_msg(name, &cached_packages, &compatible_packages_to_download);
        pacm_logger::finish(&msg);
        Ok(())
    }

    async fn install_batch_async(
        &self,
        project_dir: &str,
        packages: &[(String, String)], // (name, version_range) pairs
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        _force: bool,
        debug: bool,
    ) -> Result<()> {
        let package_names: Vec<&str> = packages.iter().map(|(name, _)| name.as_str()).collect();
        pacm_logger::status(&format!("Installing {}", package_names.join(" ")));

        let path = PathBuf::from(project_dir);

        let mut existing_packages = Vec::new();
        let mut packages_to_install = Vec::new();

        for (name, version_range) in packages {
            if self.check_existing(
                &path,
                name,
                version_range,
                dep_type,
                save_exact,
                no_save,
                debug,
            )? {
                existing_packages.push((name.clone(), version_range.clone()));
            } else {
                packages_to_install.push((name.clone(), version_range.clone()));
            }
        }

        if packages_to_install.is_empty() {
            pacm_logger::finish("All packages are already installed");
            return Ok(());
        }

        if debug && !existing_packages.is_empty() {
            pacm_logger::debug(
                &format!(
                    "Skipping {} already installed packages",
                    existing_packages.len()
                ),
                debug,
            );
        }

        self.cache.build_index(debug).await?;

        let start_fast_check = std::time::Instant::now();
        let all_cached = self.cache.are_all_cached(&packages_to_install).await;

        if debug {
            pacm_logger::debug(
                &format!(
                    "Fast cache check completed in {:?}",
                    start_fast_check.elapsed()
                ),
                debug,
            );
        }

        if all_cached {
            if debug {
                pacm_logger::debug(
                    "All packages found in cache - using fast installation path",
                    debug,
                );
            }
            return self
                .install_batch_fast_cached(
                    &path,
                    &packages_to_install,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await;
        }

        if debug {
            pacm_logger::debug(
                "Some packages not cached - using full resolution path",
                debug,
            );
        }

        let (cached_packages, packages_to_download, direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(&packages_to_install, false, &self.cache, debug)
            .await?;

        let compatible_packages_to_download: Vec<ResolvedPackage> = packages_to_download
            .iter()
            .filter(|pkg| {
                if is_platform_compatible(&pkg.os, &pkg.cpu) {
                    true
                } else {
                    pacm_logger::warn(&format!(
                        "Package {} (version {}) is not compatible with current platform, skipping",
                        pkg.name, pkg.version
                    ));
                    false
                }
            })
            .cloned()
            .collect();

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !compatible_packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&compatible_packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);
        }

        self.link_all_to_project(&path, &stored_packages, debug)?;

        if !stored_packages.is_empty() {
            super::utils::InstallUtils::run_postinstall_in_project(&path, &stored_packages, debug)?;
        }

        if !no_save {
            self.update_package_json_batch(
                &path,
                &packages_to_install,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(&path, &stored_packages, &direct_names)?;

        let finish_msg = self.build_batch_finish_msg(
            &packages_to_install,
            &cached_packages,
            &packages_to_download,
            &stored_packages,
        );
        pacm_logger::finish(&finish_msg);

        Ok(())
    }

    async fn install_batch_fast_cached(
        &self,
        path: &PathBuf,
        packages_to_install: &[(String, String)],
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug(
                "Using optimized fast-cached installation path with transitive dependencies",
                debug,
            );
        }

        let (_, all_resolved) = self
            .resolver
            .resolve_all_parallel(packages_to_install, false, debug)
            .await?;

        let (cached_packages, packages_to_download) = self
            .resolver
            .separate_cached_fast(&all_resolved, &self.cache, debug)
            .await?;

        if !packages_to_download.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "Some transitive dependencies not cached ({}), falling back to full resolution",
                        packages_to_download.len()
                    ),
                    debug,
                );
            }

            return self
                .install_batch_full_resolution(
                    path,
                    packages_to_install,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await;
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "All {} packages (including transitive deps) found in cache",
                    cached_packages.len()
                ),
                debug,
            );
        }

        let stored_packages = self.build_stored_map(&cached_packages, &all_resolved);

        self.link_all_to_project(path, &stored_packages, debug)?;

        super::utils::InstallUtils::run_postinstall_in_project(path, &stored_packages, debug)?;

        let direct_names: Vec<String> = packages_to_install
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        if !no_save {
            self.update_package_json_batch(
                path,
                packages_to_install,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(
            path,
            &stored_packages,
            &direct_names.iter().cloned().collect(),
        )?;

        let finish_msg = self.build_batch_finish_msg(
            packages_to_install,
            &cached_packages,
            &[],
            &stored_packages,
        );
        pacm_logger::finish(&finish_msg);

        Ok(())
    }

    async fn install_batch_full_resolution(
        &self,
        path: &PathBuf,
        packages_to_install: &[(String, String)],
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        let (cached_packages, packages_to_download, direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(packages_to_install, false, &self.cache, debug)
            .await?;

        let compatible_packages_to_download: Vec<ResolvedPackage> = packages_to_download
            .iter()
            .filter(|pkg| {
                if is_platform_compatible(&pkg.os, &pkg.cpu) {
                    true
                } else {
                    pacm_logger::warn(&format!(
                        "Package {} (version {}) is not compatible with current platform, skipping",
                        pkg.name, pkg.version
                    ));
                    false
                }
            })
            .cloned()
            .collect();

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !compatible_packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&compatible_packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);

            self.run_post_install(&stored_packages, &compatible_packages_to_download, debug)?;
        }

        self.link_all_to_project(path, &stored_packages, debug)?;

        if !no_save {
            self.update_package_json_batch(
                path,
                packages_to_install,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(
            path,
            &stored_packages,
            &direct_names.iter().cloned().collect(),
        )?;

        let finish_msg = self.build_batch_finish_msg(
            packages_to_install,
            &cached_packages,
            &packages_to_download,
            &stored_packages,
        );
        pacm_logger::finish(&finish_msg);

        Ok(())
    }

    fn check_existing(
        &self,
        path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<bool> {
        super::utils::InstallUtils::check_existing(
            path,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            debug,
        )
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
        direct_names: &HashSet<String>,
    ) -> Result<()> {
        let lock_path = path.join("pacm.lock");
        self.linker
            .update_lock_direct(&lock_path, stored, direct_names)
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

    fn update_package_json(
        &self,
        path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        super::utils::InstallUtils::update_pkg_json(
            path,
            name,
            version_range,
            dep_type,
            save_exact,
            stored_packages,
        )
    }

    fn update_package_json_batch(
        &self,
        path: &PathBuf,
        packages: &[(String, String)],
        dep_type: DependencyType,
        save_exact: bool,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        for (name, version_range) in packages {
            self.update_package_json(
                path,
                name,
                version_range,
                dep_type,
                save_exact,
                stored_packages,
            )?;
        }
        Ok(())
    }

    fn build_finish_msg(
        &self,
        name: &str,
        cached: &[CachedPackage],
        downloaded: &[ResolvedPackage],
    ) -> String {
        let cached_count = cached.len();
        let downloaded_count = downloaded.len();
        let total_count = cached_count + downloaded_count;

        if total_count == 1 {
            if cached_count == 1 {
                format!("{} linked from cache", name)
            } else {
                format!("{} downloaded and installed", name)
            }
        } else if cached_count > 0 && downloaded_count > 0 {
            format!(
                "{} and {} dependencies installed ({} from cache, {} downloaded)",
                name,
                total_count - 1,
                cached_count,
                downloaded_count
            )
        } else if cached_count > 0 {
            format!(
                "{} and {} dependencies linked from cache",
                name,
                cached_count - 1
            )
        } else {
            format!(
                "{} and {} dependencies downloaded and installed",
                name,
                downloaded_count - 1
            )
        }
    }

    fn build_batch_finish_msg(
        &self,
        packages_to_install: &[(String, String)],
        _cached_packages: &[CachedPackage],
        _packages_to_download: &[ResolvedPackage],
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> String {
        let mut installed_packages = Vec::new();

        for (name, _) in packages_to_install {
            for (key, (resolved_pkg, _)) in stored_packages {
                if key.starts_with(&format!("{}@", name)) {
                    installed_packages
                        .push(format!("{}@{}", resolved_pkg.name, resolved_pkg.version));
                    break;
                }
            }
        }

        if installed_packages.is_empty() {
            return "Installation completed".to_string();
        }

        let total_deps = stored_packages.len();
        let _direct_count = packages_to_install.len();

        let mut lines = Vec::new();

        for installed in &installed_packages {
            lines.push(format!("✓ installed {}", installed));
        }

        lines.push(format!("{} packages installed", total_deps));

        lines.join("\n")
    }

    async fn install_full_path(
        &self,
        project_path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug("Package not in store - using full resolution path", debug);
        } else {
            pacm_logger::status(&format!("Analyzing package requirements for {}...", name));
        }

        let deps = vec![(name.to_string(), version_range.to_string())];
        self.cache.build_index(debug).await?;

        let (cached_packages, packages_to_download, direct_names, all_resolved_packages) = {
            let (direct_names, resolved_map) = self
                .resolver
                .resolve_all_parallel(&deps, false, debug)
                .await?;

            let (cached, to_download) = self
                .resolver
                .separate_cached_fast(&resolved_map, &self.cache, debug)
                .await?;

            (cached, to_download, direct_names, resolved_map)
        };

        let compatible_packages_to_download: Vec<ResolvedPackage> = packages_to_download
            .iter()
            .filter(|pkg| {
                if is_platform_compatible(&pkg.os, &pkg.cpu) {
                    true
                } else {
                    pacm_logger::warn(&format!(
                        "Package {} (version {}) is not compatible with current platform, skipping",
                        pkg.name, pkg.version
                    ));
                    false
                }
            })
            .cloned()
            .collect();

        let mut stored_packages = self.build_stored_map(&cached_packages, &all_resolved_packages);

        if compatible_packages_to_download.is_empty() && !cached_packages.is_empty() {
            if debug {
                pacm_logger::debug(
                    &format!("All {} packages found in cache", cached_packages.len()),
                    debug,
                );
            }

            self.link_all_to_project(project_path, &stored_packages, debug)?;

            super::utils::InstallUtils::run_postinstall_in_project(
                project_path,
                &stored_packages,
                debug,
            )?;

            if !no_save {
                self.update_package_json(
                    project_path,
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    &stored_packages,
                )?;
            }

            self.update_lock(project_path, &stored_packages, &direct_names)?;

            let msg = if cached_packages.len() == 1 {
                format!("{} linked from cache", name)
            } else {
                format!(
                    "{} and {} dependencies linked from cache",
                    name,
                    cached_packages.len() - 1
                )
            };
            pacm_logger::finish(&msg);
            return Ok(());
        }

        if !compatible_packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&compatible_packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);

            self.run_post_install(&stored_packages, &compatible_packages_to_download, debug)?;
        }

        self.link_all_to_project(project_path, &stored_packages, debug)?;

        if !no_save {
            self.update_package_json(
                project_path,
                name,
                version_range,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(project_path, &stored_packages, &direct_names)?;

        let msg = self.build_finish_msg(name, &cached_packages, &compatible_packages_to_download);
        pacm_logger::finish(&msg);
        Ok(())
    }
}

impl Default for SingleInstaller {
    fn default() -> Self {
        Self::new()
    }
}
