use rayon::prelude::*;
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

    pub fn run_postinstall_in_project(
        project_dir: &PathBuf,
        packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Running postinstall scripts for {} packages in project node_modules",
                    packages.len()
                ),
                debug,
            );
        }

        let project_node_modules = project_dir.join("node_modules");

        let results: Vec<_> = packages
            .par_iter()
            .map(|(_key, (pkg, _store_path))| {
                Self::run_single_postinstall_in_project(&pkg.name, &project_node_modules, debug)
            })
            .collect();

        for result in results {
            result?;
        }

        let temp_dir = project_dir.join(".pacm_temp");
        if temp_dir.exists() {
            if debug {
                pacm_logger::debug(
                    &format!("Cleaning up temporary directory: {}", temp_dir.display()),
                    debug,
                );
            }
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                pacm_logger::warn(&format!(
                    "Failed to clean up temporary directory {}: {}",
                    temp_dir.display(),
                    e
                ));
            } else if debug {
                pacm_logger::debug("Successfully cleaned up .pacm_temp directory", debug);
            }
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
                pacm_logger::status(&format!(
                    "Running postinstall for {} in directory: {}",
                    package_name,
                    package_dir.display()
                ));

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

    fn run_single_postinstall_in_project(
        package_name: &str,
        project_node_modules: &PathBuf,
        debug: bool,
    ) -> Result<()> {
        let package_dir = if package_name.starts_with('@') {
            if let Some(slash_pos) = package_name.find('/') {
                let scope = &package_name[..slash_pos]; // @types
                let name = &package_name[slash_pos + 1..]; // node
                let scope_dir = project_node_modules.join(scope);
                scope_dir.join(name)
            } else {
                project_node_modules.join(package_name)
            }
        } else {
            project_node_modules.join(package_name)
        };

        let package_json_path = package_dir.join("package.json");

        if !package_json_path.exists() {
            if debug {
                pacm_logger::debug(
                    &format!(
                        "No package.json found for {} in project node_modules",
                        package_name
                    ),
                    debug,
                );
            }
            return Ok(());
        }

        let content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let package_json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        if let Some(scripts) = package_json.get("scripts").and_then(|s| s.as_object()) {
            if let Some(postinstall) = scripts.get("postinstall").and_then(|s| s.as_str()) {
                pacm_logger::status(&format!(
                    "Running postinstall for {} in project directory: {}",
                    package_name,
                    package_dir.display()
                ));

                if debug {
                    pacm_logger::debug(
                        &format!(
                            "Running postinstall for {} in project: {}",
                            package_name, postinstall
                        ),
                        debug,
                    );
                }

                let project_root = project_node_modules
                    .parent()
                    .unwrap_or(project_node_modules);

                let temp_package_dir = project_root
                    .join(".pacm_temp")
                    .join(package_name.replace("/", "_"));

                if temp_package_dir.exists() {
                    let _ = std::fs::remove_dir_all(&temp_package_dir);
                }

                if let Err(e) = std::fs::create_dir_all(&temp_package_dir) {
                    pacm_logger::warn(&format!(
                        "Failed to create temp directory for {}: {}",
                        package_name, e
                    ));
                    return Ok(());
                }

                let store_package_dir = package_dir.read_link().unwrap_or(package_dir.clone());
                if let Err(e) = Self::copy_dir_contents(&store_package_dir, &temp_package_dir) {
                    pacm_logger::warn(&format!(
                        "Failed to copy package contents for {}: {}",
                        package_name, e
                    ));
                    let _ = std::fs::remove_dir_all(&temp_package_dir);
                    return Ok(());
                }

                let temp_node_modules = temp_package_dir.join("node_modules");
                if let Err(e) = std::fs::create_dir_all(&temp_node_modules) {
                    pacm_logger::warn(&format!(
                        "Failed to create temp node_modules for {}: {}",
                        package_name, e
                    ));
                    let _ = std::fs::remove_dir_all(&temp_package_dir);
                    return Ok(());
                }

                if let Ok(entries) = std::fs::read_dir(project_node_modules) {
                    for entry in entries.flatten() {
                        let entry_name = entry.file_name();
                        let entry_name_str = entry_name.to_string_lossy();
                        let temp_link = temp_node_modules.join(&entry_name);

                        if temp_link.exists() || entry_name_str == package_name {
                            continue;
                        }

                        #[cfg(target_family = "windows")]
                        {
                            if entry.path().is_dir() {
                                let _ = std::os::windows::fs::symlink_dir(entry.path(), temp_link);
                            } else {
                                let _ = std::os::windows::fs::symlink_file(entry.path(), temp_link);
                            }
                        }

                        #[cfg(target_family = "unix")]
                        {
                            let _ = std::os::unix::fs::symlink(entry.path(), temp_link);
                        }
                    }
                }

                let self_link = temp_node_modules.join(package_name);
                if !self_link.exists() {
                    #[cfg(target_family = "windows")]
                    {
                        let _ = std::os::windows::fs::symlink_dir(&temp_package_dir, self_link);
                    }

                    #[cfg(target_family = "unix")]
                    {
                        let _ = std::os::unix::fs::symlink(&temp_package_dir, self_link);
                    }
                }

                let mut cmd = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                } else {
                    Command::new("sh")
                };

                if cfg!(target_os = "windows") {
                    cmd.args(["/C", postinstall]);
                } else {
                    cmd.args(["-c", postinstall]);
                }

                cmd.current_dir(&temp_package_dir);

                cmd.env("NODE_PATH", temp_node_modules.to_string_lossy().as_ref());
                cmd.env("npm_package_name", package_name);
                cmd.env("INIT_CWD", project_root.to_string_lossy().as_ref());

                if let Some(version) = package_json.get("version").and_then(|v| v.as_str()) {
                    cmd.env("npm_package_version", version);
                }

                if let Some(path) = std::env::var_os("PATH") {
                    let mut paths = std::env::split_paths(&path).collect::<Vec<_>>();
                    paths.insert(0, project_node_modules.join(".bin"));
                    let new_path = std::env::join_paths(paths).unwrap();
                    cmd.env("PATH", new_path);
                }

                let status = cmd.status();

                let _ = std::fs::remove_dir_all(&temp_package_dir);

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
                                    "Postinstall script completed successfully for {} in project",
                                    package_name
                                ),
                                debug,
                            );
                        }
                    }
                    Err(e) => {
                        pacm_logger::warn(&format!(
                            "Failed to execute postinstall script for {} in project: {}",
                            package_name, e
                        ));
                    }
                }
            }
        } else if debug {
            pacm_logger::debug(
                &format!("No postinstall script found for {}", package_name),
                debug,
            );
        }

        Ok(())
    }

    fn copy_dir_contents(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
        if !src.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source is not a directory",
            ));
        }

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                std::fs::create_dir_all(&dst_path)?;
                Self::copy_dir_contents(&src_path, &dst_path)?;
            } else {
                if let Some(parent) = dst_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src_path, &dst_path)?;
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
