use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::cache::CacheManager;
use super::resolver::DependencyResolver;
use super::types::CachedPackage;
use crate::download::PackageDownloader;
use crate::linker::PackageLinker;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_project::DependencyType;
use pacm_resolver::ResolvedPackage;

pub struct SingleInstaller {
    downloader: PackageDownloader,
    linker: PackageLinker,
    cache: CacheManager,
    resolver: DependencyResolver,
}

impl SingleInstaller {
    pub fn new() -> Self {
        Self {
            downloader: PackageDownloader::new(),
            linker: PackageLinker {},
            cache: CacheManager::new(),
            resolver: DependencyResolver::new(),
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
        pacm_logger::status(&format!("Installing {}@{}", name, version_range));

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

        if debug {
            pacm_logger::debug("Checking store for instant installation...", debug);
        }

        if let Some(cached_package) = self.fast_store_check(name, version_range, debug).await? {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "Found {}@{} in store - using fast path",
                        name, version_range
                    ),
                    debug,
                );
            }
            return self
                .fast_link_only(
                    &path,
                    &cached_package,
                    name,
                    version_range,
                    dep_type,
                    save_exact,
                    no_save,
                    debug,
                )
                .await;
        }

        if debug {
            pacm_logger::debug("Package not in store - using full resolution path", debug);
        } else {
            pacm_logger::status(&format!("Analyzing package requirements for {}...", name));
        }

        let deps = vec![(name.to_string(), version_range.to_string())];
        self.cache.build_index(debug).await?;

        let (cached_packages, packages_to_download, mut direct_names, resolved_map) = self
            .resolver
            .resolve_deps_optimized(&deps, false, &self.cache, debug)
            .await?;

        direct_names.clear();
        direct_names.insert(name.to_string());

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if packages_to_download.is_empty() && !cached_packages.is_empty() {
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

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&packages_to_download, debug)
                .await?;
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

        let msg = self.build_finish_msg(name, &cached_packages, &packages_to_download);
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

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);

            self.link_store_deps(&stored_packages, debug)?;
            self.run_post_install(&stored_packages, &packages_to_download, debug)?;
        }

        self.link_to_project(&path, &stored_packages, &direct_names, debug)?;

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
            pacm_logger::debug("Using optimized fast-cached installation path", debug);
        }

        let cached_packages = self
            .cache
            .get_batch_direct(packages_to_install)
            .await
            .into_iter()
            .filter_map(|cached_opt| cached_opt)
            .collect::<Vec<_>>();

        if cached_packages.len() != packages_to_install.len() {
            if debug {
                pacm_logger::debug(
                    "Cache miss detected, falling back to full resolution",
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

        let mut stored_packages = HashMap::new();
        let mut direct_names = HashSet::new();

        for cached_pkg in &cached_packages {
            let key = format!("{}@{}", cached_pkg.name, cached_pkg.version);
            let resolved_pkg = ResolvedPackage {
                name: cached_pkg.name.clone(),
                version: cached_pkg.version.clone(),
                resolved: cached_pkg.resolved.clone(),
                integrity: cached_pkg.integrity.clone(),
                dependencies: HashMap::new(), // We don't need dependencies for direct packages in fast path
            };
            stored_packages.insert(key, (resolved_pkg, cached_pkg.store_path.clone()));
            direct_names.insert(cached_pkg.name.clone());
        }

        if debug {
            pacm_logger::debug(
                "Skipping dependency verification for cached packages in fast path",
                debug,
            );
        }

        self.link_to_project(path, &stored_packages, &direct_names, debug)?;

        if !no_save {
            self.update_package_json_batch(
                path,
                packages_to_install,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(path, &stored_packages, &direct_names)?;

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

        let mut stored_packages = self.build_stored_map(&cached_packages, &resolved_map);

        if !cached_packages.is_empty() {
            self.link_cached_deps(&cached_packages, &stored_packages, debug)?;
        }

        if !packages_to_download.is_empty() {
            let downloaded = self
                .downloader
                .download_parallel(&packages_to_download, debug)
                .await?;
            stored_packages.extend(downloaded);

            self.link_store_deps(&stored_packages, debug)?;
            self.run_post_install(&stored_packages, &packages_to_download, debug)?;
        }

        self.link_to_project(path, &stored_packages, &direct_names, debug)?;

        if !no_save {
            self.update_package_json_batch(
                path,
                packages_to_install,
                dep_type,
                save_exact,
                &stored_packages,
            )?;
        }

        self.update_lock(path, &stored_packages, &direct_names)?;

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

    async fn fast_store_check(
        &self,
        name: &str,
        version_range: &str,
        debug: bool,
    ) -> Result<Option<CachedPackage>> {
        use pacm_registry::fetch_package_info_async;
        use pacm_resolver::semver::resolve_version;
        use pacm_store::get_store_path;

        let resolved_version = if version_range.starts_with('^')
            || version_range.starts_with('~')
            || version_range.contains('x')
            || version_range.contains('*')
        {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "Complex version range '{}' - skipping fast path",
                        version_range
                    ),
                    debug,
                );
            }
            return Ok(None);
        } else if version_range == "latest"
            || !version_range.chars().next().unwrap_or('0').is_ascii_digit()
        {
            if debug {
                pacm_logger::debug(
                    &format!("Resolving version tag '{}' for {}", version_range, name),
                    debug,
                );
            }

            let client = self.resolver.get_client();
            match fetch_package_info_async(client, name).await {
                Ok(pkg_info) => {
                    match resolve_version(&pkg_info.versions, version_range, &pkg_info.dist_tags) {
                        Ok(version) => {
                            if debug {
                                pacm_logger::debug(
                                    &format!("Resolved '{}' to version {}", version_range, version),
                                    debug,
                                );
                            }
                            version
                        }
                        Err(e) => {
                            if debug {
                                pacm_logger::debug(
                                    &format!(
                                        "Failed to resolve version '{}': {}",
                                        version_range, e
                                    ),
                                    debug,
                                );
                            }
                            return Ok(None);
                        }
                    }
                }
                Err(e) => {
                    if debug {
                        pacm_logger::debug(
                            &format!("Failed to fetch package info for {}: {}", name, e),
                            debug,
                        );
                    }
                    return Ok(None);
                }
            }
        } else {
            version_range.to_string()
        };

        let store_base = get_store_path();
        let npm_dir = store_base.join("npm");

        if !npm_dir.exists() {
            return Ok(None);
        }

        let safe_package_name = if name.starts_with('@') {
            name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            name.to_string()
        };

        let package_dir = npm_dir.join(&safe_package_name);

        if debug {
            pacm_logger::debug(
                &format!(
                    "Fast checking store for package: {} version: {}",
                    safe_package_name, resolved_version
                ),
                debug,
            );
        }

        if package_dir.exists() {
            let version_dir = package_dir.join(&resolved_version);
            let package_path = version_dir.join("package");

            if package_path.exists() {
                if debug {
                    pacm_logger::debug(
                        &format!("Fast store hit: {}/{}", safe_package_name, resolved_version),
                        debug,
                    );
                }

                return Ok(Some(CachedPackage {
                    name: name.to_string(),
                    version: resolved_version.clone(),
                    resolved: format!(
                        "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                        name,
                        name.split('/').last().unwrap_or(name),
                        resolved_version
                    ),
                    integrity: String::new(), // Will be filled if needed
                    store_path: version_dir,
                }));
            }
        }

        Ok(None)
    }

    async fn fast_link_only(
        &self,
        project_path: &PathBuf,
        cached_package: &CachedPackage,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<()> {
        if debug {
            pacm_logger::debug("Using fast link-only path - no downloads needed", debug);
        }

        let mut stored_packages = HashMap::new();
        let key = format!("{}@{}", cached_package.name, cached_package.version);
        let resolved_pkg = ResolvedPackage {
            name: cached_package.name.clone(),
            version: cached_package.version.clone(),
            resolved: cached_package.resolved.clone(),
            integrity: cached_package.integrity.clone(),
            dependencies: HashMap::new(), // Single package - no dependencies needed
        };
        stored_packages.insert(key, (resolved_pkg, cached_package.store_path.clone()));

        let mut direct_names = HashSet::new();
        direct_names.insert(name.to_string());

        self.link_to_project(project_path, &stored_packages, &direct_names, debug)?;

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

        pacm_logger::finish(&format!("{} linked instantly from store", name));
        Ok(())
    }
}

impl Default for SingleInstaller {
    fn default() -> Self {
        Self::new()
    }
}
