use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use crate::download::PackageDownloader;
use crate::linker::PackageLinker;
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::{DependencyType, read_package_json};
use pacm_resolver::{ResolvedPackage, resolve_full_tree};
use pacm_error::{PackageManagerError, Result};
use pacm_store::get_store_path;

pub struct InstallManager {
    downloader: PackageDownloader,
    linker: PackageLinker,
}

#[derive(Debug, Clone)]
struct CachedPackage {
    name: String,
    version: String,
    resolved: String,
    integrity: String,
    store_path: PathBuf,
}

enum PackageSource {
    Cache(CachedPackage),
    Download(ResolvedPackage),
}

impl InstallManager {
    pub fn new() -> Self {
        Self {
            downloader: PackageDownloader::new(),
            linker: PackageLinker,
        }
    }

    pub fn install_all_dependencies(&self, project_dir: &str, debug: bool) -> Result<()> {
        let path = PathBuf::from(project_dir);
        let pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;
        let lock_path = path.join("pacm.lock");

        let (direct_deps, use_lockfile) = if lock_path.exists() {
            pacm_logger::status("Using existing lockfile for installation...");
            let lockfile = PacmLock::load(&lock_path)
                .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

            let deps: Vec<(String, String)> = lockfile
                .dependencies
                .iter()
                .map(|(name, lock_dep)| (name.clone(), lock_dep.version.clone()))
                .collect();
            (deps, true)
        } else {
            pacm_logger::status("No lockfile found, using package.json dependencies...");
            let all_deps = pkg.get_all_dependencies();
            let deps: Vec<(String, String)> = all_deps.into_iter().collect();
            (deps, false)
        };

        if direct_deps.is_empty() {
            pacm_logger::finish("No dependencies to install");
            return Ok(());
        }

        let (cached_packages, packages_to_download, direct_package_names) =
            self.smart_resolve_packages(&direct_deps, use_lockfile, debug)?;

        let mut stored_packages = HashMap::new();
        for cached_pkg in &cached_packages {
            stored_packages.insert(
                format!("{}@{}", cached_pkg.name, cached_pkg.version),
                (
                    ResolvedPackage {
                        name: cached_pkg.name.clone(),
                        version: cached_pkg.version.clone(),
                        resolved: cached_pkg.resolved.clone(),
                        integrity: cached_pkg.integrity.clone(),
                        dependencies: HashMap::new(),
                    },
                    cached_pkg.store_path.clone(),
                ),
            );
        }

        if !packages_to_download.is_empty() {
            pacm_logger::status(&format!(
                "Downloading {} packages...",
                packages_to_download.len()
            ));
            let downloaded_packages = self
                .downloader
                .download_packages(&packages_to_download, debug)?;
            stored_packages.extend(downloaded_packages);
        }

        self.linker.link_direct_dependencies_to_project(
            &path,
            &stored_packages,
            &direct_package_names,
            debug,
        )?;

        if !packages_to_download.is_empty() {
            let new_packages: HashMap<String, (ResolvedPackage, PathBuf)> = stored_packages
                .iter()
                .filter(|(key, _)| {
                    packages_to_download
                        .iter()
                        .any(|pkg| key.starts_with(&format!("{}@", pkg.name)))
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            self.run_postinstall_scripts(&new_packages, debug)?;
        }

        self.linker.update_lockfile_direct_only(
            &lock_path,
            &stored_packages,
            &direct_package_names,
        )?;

        let cached_count = cached_packages.len();
        let downloaded_count = packages_to_download.len();
        let final_message = if cached_count > 0 && downloaded_count > 0 {
            format!(
                "{} packages installed ({} from cache, {} downloaded)",
                cached_count + downloaded_count,
                cached_count,
                downloaded_count
            )
        } else if cached_count > 0 {
            format!("{} packages linked from cache", cached_count)
        } else {
            format!("{} packages downloaded and installed", downloaded_count)
        };

        pacm_logger::finish(&final_message);
        Ok(())
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
        let path = PathBuf::from(project_dir);
        let pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;
        let lock_path = path.join("pacm.lock");

        if let Some(existing_type) = pkg.has_dependency(name) {
            self.handle_existing_dependency(name, existing_type, dep_type, force)?;
        }

        let package_source = self.check_single_package_cache(name, version_range, debug)?;

        let (stored_packages, was_cached) = match package_source {
            PackageSource::Cache(cached_pkg) => {
                pacm_logger::status(&format!("Linking {} from cache...", name));
                let mut packages = HashMap::new();
                packages.insert(
                    format!("{}@{}", cached_pkg.name, cached_pkg.version),
                    (
                        ResolvedPackage {
                            name: cached_pkg.name.clone(),
                            version: cached_pkg.version.clone(),
                            resolved: cached_pkg.resolved.clone(),
                            integrity: cached_pkg.integrity.clone(),
                            dependencies: HashMap::new(),
                        },
                        cached_pkg.store_path.clone(),
                    ),
                );
                (packages, true)
            }
            PackageSource::Download(_main_pkg) => {
                pacm_logger::status(&format!("Resolving and downloading {}...", name));

                let mut seen = HashSet::new();
                let all_packages =
                    resolve_full_tree(name, version_range, &mut seen).map_err(|e| {
                        PackageManagerError::VersionResolutionFailed(
                            name.to_string(),
                            e.to_string(),
                        )
                    })?;

                let downloaded_packages =
                    self.downloader.download_packages(&all_packages, debug)?;

                self.linker
                    .link_dependencies_to_store(&downloaded_packages, debug)?;

                self.run_postinstall_scripts(&downloaded_packages, debug)?;

                (downloaded_packages, false)
            }
        };

        self.linker
            .link_single_package_to_project(&path, name, &stored_packages, debug)?;

        if !no_save {
            if let Some((pkg_resolved, _)) = stored_packages
                .iter()
                .find(|(key, _)| key.starts_with(&format!("{}@", name)))
                .map(|(_, (pkg, store_path))| (pkg, store_path))
            {
                self.linker.update_package_json(
                    &path,
                    &pkg_resolved.name,
                    &pkg_resolved.version,
                    dep_type,
                    save_exact,
                )?;
            }
        }

        let main_package_names = HashSet::from([name.to_string()]);
        self.linker.update_lockfile_direct_only(
            &lock_path,
            &stored_packages,
            &main_package_names,
        )?;

        let main_package = stored_packages
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{}@", name)))
            .map(|(_, (pkg, _))| format!("{}@{}", pkg.name, pkg.version))
            .unwrap_or_else(|| format!("{}@unknown", name));

        let final_message = if was_cached {
            format!("linked {} from cache", main_package)
        } else {
            let installed_count = stored_packages.len();
            if installed_count == 1 {
                format!("installed {}", main_package)
            } else {
                format!(
                    "installed {} (with {} dependencies)",
                    main_package,
                    installed_count - 1
                )
            }
        };

        pacm_logger::finish(&final_message);
        Ok(())
    }

    fn handle_existing_dependency(
        &self,
        name: &str,
        existing_type: DependencyType,
        new_type: DependencyType,
        force: bool,
    ) -> Result<()> {
        match (existing_type, new_type) {
            (DependencyType::Dependencies, DependencyType::DevDependencies) => {
                pacm_logger::status(&format!(
                    "Moving {} from dependencies to devDependencies...",
                    name
                ));
            }
            (DependencyType::DevDependencies, DependencyType::Dependencies) => {
                pacm_logger::status(&format!(
                    "Moving {} from devDependencies to dependencies...",
                    name
                ));
            }
            _ => {
                if !force {
                    pacm_logger::status(&format!("{} is already installed", name));
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn run_postinstall_scripts(
        &self,
        stored_packages: &std::collections::HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        let scripts_to_run: Vec<_> = stored_packages
            .iter()
            .filter_map(|(_package_key, (pkg, store_path))| {
                let package_json_path = store_path.join("package").join("package.json");
                if package_json_path.exists() {
                    let file = std::fs::File::open(&package_json_path).ok()?;
                    let pkg_data: serde_json::Value = serde_json::from_reader(file).ok()?;
                    let script = pkg_data
                        .get("scripts")
                        .and_then(|s| s.get("postinstall"))
                        .and_then(|s| s.as_str())?;
                    Some((pkg.name.clone(), script.to_string(), store_path.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (pkg_name, script, store_path) in scripts_to_run {
            pacm_logger::status(&format!("Running postinstall for {}...", pkg_name));

            let status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", &script])
                    .current_dir(&store_path.join("package"))
                    .status()
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(&script)
                    .current_dir(&store_path.join("package"))
                    .status()
            };

            match status {
                Ok(status) if !status.success() => {
                    pacm_logger::warn(&format!("Postinstall script for {} failed", pkg_name));
                    pacm_logger::debug(
                        &format!("Postinstall script failed for {}", pkg_name),
                        debug,
                    );
                }
                Err(e) => {
                    pacm_logger::error(&format!(
                        "Failed to run postinstall for {}: {}",
                        pkg_name, e
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn smart_resolve_packages(
        &self,
        direct_deps: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<(Vec<CachedPackage>, Vec<ResolvedPackage>, HashSet<String>)> {
        pacm_logger::status("Checking package cache...");

        let mut cached_packages = Vec::new();
        let mut packages_to_download = Vec::new();
        let mut direct_package_names = HashSet::new();

        for (name, version_or_range) in direct_deps {
            direct_package_names.insert(name.clone());

            if use_lockfile {
                if let Some(cached_pkg) = self.check_store_cache(name, version_or_range, debug)? {
                    pacm_logger::debug(&format!("Found {} in cache", name), debug);
                    cached_packages.push(cached_pkg);
                } else {
                    pacm_logger::debug(&format!("Need to download {}", name), debug);
                    packages_to_download.push(ResolvedPackage {
                        name: name.clone(),
                        version: version_or_range.clone(),
                        resolved: format!(
                            "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                            name, name, version_or_range
                        ),
                        integrity: String::new(),
                        dependencies: HashMap::new(),
                    });
                }
            } else {
                let mut seen = HashSet::new();
                match resolve_full_tree(name, version_or_range, &mut seen) {
                    Ok(resolved_packages) => {
                        if let Some(main_pkg) = resolved_packages.first() {
                            if let Some(cached_pkg) =
                                self.check_store_cache(&main_pkg.name, &main_pkg.version, debug)?
                            {
                                pacm_logger::debug(&format!("Found {} in cache", name), debug);
                                cached_packages.push(cached_pkg);
                            } else {
                                pacm_logger::debug(&format!("Need to download {}", name), debug);
                                packages_to_download.extend(resolved_packages);
                            }
                        }
                    }
                    Err(e) => {
                        pacm_logger::error(&format!(
                            "Failed to resolve {}@{}: {}",
                            name, version_or_range, e
                        ));
                        return Err(PackageManagerError::VersionResolutionFailed(
                            name.clone(),
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        Ok((cached_packages, packages_to_download, direct_package_names))
    }

    fn check_single_package_cache(
        &self,
        name: &str,
        version_range: &str,
        debug: bool,
    ) -> Result<PackageSource> {
        let mut seen = HashSet::new();
        let resolved_packages = resolve_full_tree(name, version_range, &mut seen).map_err(|e| {
            PackageManagerError::VersionResolutionFailed(name.to_string(), e.to_string())
        })?;

        if let Some(main_pkg) = resolved_packages.first() {
            if let Some(cached_pkg) =
                self.check_store_cache(&main_pkg.name, &main_pkg.version, debug)?
            {
                Ok(PackageSource::Cache(cached_pkg))
            } else {
                Ok(PackageSource::Download(main_pkg.clone()))
            }
        } else {
            Err(PackageManagerError::VersionResolutionFailed(
                name.to_string(),
                "No packages resolved".to_string(),
            ))
        }
    }

    fn check_store_cache(
        &self,
        name: &str,
        version: &str,
        debug: bool,
    ) -> Result<Option<CachedPackage>> {
        let store_base = get_store_path();

        let safe_package_name = if name.starts_with('@') {
            name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            name.to_string()
        };

        let npm_dir = store_base.join("npm");
        if !npm_dir.exists() {
            return Ok(None);
        }

        match std::fs::read_dir(&npm_dir) {
            Ok(entries) => {
                let package_prefix = format!("{safe_package_name}@{version}-");

                for entry in entries {
                    if let Ok(entry) = entry {
                        let dir_name = entry.file_name();
                        if let Some(name_str) = dir_name.to_str() {
                            if name_str.starts_with(&package_prefix) {
                                let store_path = entry.path();
                                if store_path.is_dir() && store_path.join("package").exists() {
                                    pacm_logger::debug(
                                        &format!("Found cached package: {}", name_str),
                                        debug,
                                    );

                                    let hash = name_str.strip_prefix(&package_prefix).unwrap_or("");

                                    return Ok(Some(CachedPackage {
                                        name: name.to_string(),
                                        version: version.to_string(),
                                        resolved: format!(
                                            "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                            name, name, version
                                        ),
                                        integrity: format!("sha256-{}", hash),
                                        store_path,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => return Ok(None),
        }

        Ok(None)
    }
}

impl Default for InstallManager {
    fn default() -> Self {
        Self::new()
    }
}
