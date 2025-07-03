use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use pacm_error::{PackageManagerError, Result};
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::{DependencyType, read_package_json, write_package_json};
use pacm_resolver::ResolvedPackage;

pub struct InstallUtils;

impl InstallUtils {
    pub fn check_existing(
        path: &PathBuf,
        name: &str,
        _version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        debug: bool,
    ) -> Result<bool> {
        let node_modules = path.join("node_modules");
        let package_dir = node_modules.join(name);

        if package_dir.exists() {
            let package_json_path = package_dir.join("package.json");
            if package_json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                    if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(installed_version) =
                            pkg_json.get("version").and_then(|v| v.as_str())
                        {
                            if debug {
                                pacm_logger::debug(
                                    &format!(
                                        "Found existing package {} in node_modules with version {}",
                                        name, installed_version
                                    ),
                                    debug,
                                );
                            }

                            if !no_save {
                                let mut pkg = read_package_json(path).map_err(|e| {
                                    PackageManagerError::PackageJsonError(e.to_string())
                                })?;

                                if pkg.has_dependency(name).is_none() {
                                    let version_to_save = if save_exact {
                                        installed_version.to_string()
                                    } else {
                                        format!("^{}", installed_version)
                                    };
                                    pkg.add_dependency(
                                        name,
                                        &version_to_save,
                                        dep_type,
                                        save_exact,
                                    );
                                    write_package_json(path, &pkg).map_err(|e| {
                                        PackageManagerError::PackageJsonError(e.to_string())
                                    })?;

                                    if debug {
                                        pacm_logger::debug(
                                            &format!(
                                                "Added {} to package.json with version {}",
                                                name, version_to_save
                                            ),
                                            debug,
                                        );
                                    }
                                } else {
                                    if debug {
                                        pacm_logger::debug(
                                            &format!(
                                                "Package {} already exists in package.json, not modifying version",
                                                name
                                            ),
                                            debug,
                                        );
                                    }
                                }
                            }

                            pacm_logger::finish(&format!(
                                "{} is already installed (found in node_modules)",
                                name
                            ));
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    pub fn update_pkg_json(
        path: &PathBuf,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        let mut pkg = read_package_json(path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let target_version = stored_packages
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{}@", name)))
            .map(|(_, (pkg, _))| &pkg.version)
            .map_or(version_range, |v| v);

        let version_to_save = if save_exact {
            target_version.to_string()
        } else if version_range.starts_with('^') || version_range.starts_with('~') {
            version_range.to_string()
        } else {
            format!("^{}", target_version)
        };

        pkg.add_dependency(name, &version_to_save, dep_type, save_exact);

        write_package_json(path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        Ok(())
    }

    pub fn update_pkg_json_existing(
        path: &Path,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
    ) -> Result<()> {
        let mut pkg = read_package_json(path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        pkg.remove_dependency(name);
        pkg.add_dependency(name, version_range, dep_type, false);

        write_package_json(path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        Ok(())
    }

    pub fn run_postinstall(
        packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Running postinstall scripts for {} packages",
                    packages.len()
                ),
                debug,
            );
        }

        for (_key, (pkg, store_path)) in packages {
            Self::run_single_postinstall(&pkg.name, store_path, debug)?;
        }

        Ok(())
    }

    fn run_single_postinstall(package_name: &str, store_path: &PathBuf, debug: bool) -> Result<()> {
        let package_dir = store_path.join("package");
        let package_json_path = package_dir.join("package.json");

        if !package_json_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let package_json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
            if let Some(postinstall) = scripts.get("postinstall").and_then(|s| s.as_str()) {
                if debug {
                    pacm_logger::debug(
                        &format!("Running postinstall for {}: {}", package_name, postinstall),
                        debug,
                    );
                }

                let status = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .args(["/C", postinstall])
                        .current_dir(&package_dir)
                        .status()
                } else {
                    Command::new("sh")
                        .args(["-c", postinstall])
                        .current_dir(&package_dir)
                        .status()
                };

                match status {
                    Ok(exit_status) => {
                        if !exit_status.success() {
                            pacm_logger::warn(&format!(
                                "Postinstall script failed for {} with exit code: {}",
                                package_name,
                                exit_status.code().unwrap_or(-1)
                            ));
                        } else if debug {
                            pacm_logger::debug(
                                &format!(
                                    "Postinstall script completed successfully for {}",
                                    package_name
                                ),
                                debug,
                            );
                        }
                    }
                    Err(e) => {
                        pacm_logger::warn(&format!(
                            "Failed to execute postinstall script for {}: {}",
                            package_name, e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn find_in_store(
        name: &str,
        version_range: &str,
        debug: bool,
    ) -> Result<Option<(String, PathBuf)>> {
        if debug {
            pacm_logger::debug(
                &format!(
                    "Searching store for compatible version of {}@{}",
                    name, version_range
                ),
                debug,
            );
        }

        let store_base = pacm_store::get_store_path();
        let npm_dir = store_base.join("npm");

        if !npm_dir.exists() {
            if debug {
                pacm_logger::debug("Store npm directory does not exist", debug);
            }
            return Ok(None);
        }

        let safe_package_name = if name.starts_with('@') {
            name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            name.to_string()
        };

        let package_dir = npm_dir.join(&safe_package_name);

        if !package_dir.exists() {
            if debug {
                pacm_logger::debug(&format!("Package {} not found in store", name), debug);
            }
            return Ok(None);
        }

        // For now, return the first version found. In the future, we could implement
        // version resolution here based on the version_range
        match std::fs::read_dir(&package_dir) {
            Ok(version_entries) => {
                for version_entry in version_entries.flatten() {
                    if version_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                        let version = version_entry.file_name().to_string_lossy().to_string();
                        let store_path = version_entry.path();
                        let package_path = store_path.join("package");

                        if package_path.exists() {
                            if debug {
                                pacm_logger::debug(
                                    &format!(
                                        "Found {} version {} in store at {:?}",
                                        name, version, store_path
                                    ),
                                    debug,
                                );
                            }
                            return Ok(Some((version, store_path)));
                        }
                    }
                }
            }
            Err(e) => {
                if debug {
                    pacm_logger::debug(
                        &format!("Error reading package directory for {}: {}", name, e),
                        debug,
                    );
                }
            }
        }

        if debug {
            pacm_logger::debug(
                &format!("No compatible version of {} found in store", name),
                debug,
            );
        }
        Ok(None)
    }

    pub fn check_existing_pkgs(
        path: &PathBuf,
        deps: &[(String, String)],
        use_lockfile: bool,
        debug: bool,
    ) -> Result<Vec<(String, String)>> {
        let node_modules = path.join("node_modules");
        if !node_modules.exists() {
            if debug {
                pacm_logger::debug("node_modules directory does not exist", debug);
            }
            return Ok(deps.to_vec());
        }

        let lock_path = path.join("pacm.lock");
        let lockfile = if lock_path.exists() && use_lockfile {
            Some(
                PacmLock::load(&lock_path)
                    .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?,
            )
        } else {
            None
        };

        let mut remaining_deps = Vec::new();

        for (name, version) in deps {
            let package_dir = node_modules.join(name);

            if package_dir.exists() {
                let package_json_path = package_dir.join("package.json");
                if package_json_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                        if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(installed_version) =
                                pkg_json.get("version").and_then(|v| v.as_str())
                            {
                                if let Some(ref lockfile) = lockfile {
                                    if let Some(lock_dep) = lockfile.get_dependency(name) {
                                        if lock_dep.version == *version
                                            && installed_version == *version
                                        {
                                            if debug {
                                                pacm_logger::debug(
                                                    &format!(
                                                        "Package {} already correctly installed in node_modules (verified with lockfile)",
                                                        name
                                                    ),
                                                    debug,
                                                );
                                            }
                                            continue;
                                        }
                                    }
                                } else {
                                    if debug {
                                        pacm_logger::debug(
                                            &format!(
                                                "Package {} found in node_modules with version {}",
                                                name, installed_version
                                            ),
                                            debug,
                                        );
                                    }
                                    continue;
                                }

                                if debug {
                                    pacm_logger::debug(
                                        &format!(
                                            "Package {} needs update: {} -> {}",
                                            name, installed_version, version
                                        ),
                                        debug,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            remaining_deps.push((name.clone(), version.clone()));
        }

        Ok(remaining_deps)
    }
}
