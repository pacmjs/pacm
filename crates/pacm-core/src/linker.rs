use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{PackageManagerError, Result};
use pacm_lock::{LockDependency, PacmLock};
use pacm_logger;
use pacm_project::{DependencyType, read_package_json, write_package_json};
use pacm_resolver::ResolvedPackage;
use pacm_store::link_package;

pub struct PackageLinker;

impl PackageLinker {
    pub fn link_dependencies_to_store(
        &self,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Setting up package dependencies...");

        for (_package_key, (pkg, store_path)) in stored_packages {
            pacm_logger::debug(
                &format!(
                    "Setting up dependencies for {}@{} in store",
                    pkg.name, pkg.version
                ),
                debug,
            );

            let package_node_modules = store_path.join("package").join("node_modules");

            for (dep_name, _dep_range) in &pkg.dependencies {
                if let Some((_, dep_store_path)) = stored_packages
                    .iter()
                    .find(|(key, _)| key.starts_with(&format!("{}@", dep_name)))
                    .map(|(_, (_, store_path))| ((), store_path))
                {
                    if let Err(e) = link_package(&package_node_modules, dep_name, dep_store_path) {
                        pacm_logger::debug(
                            &format!(
                                "Failed to link dependency {} for package {}: {}",
                                dep_name, pkg.name, e
                            ),
                            debug,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub fn link_direct_dependencies_to_project(
        &self,
        project_dir: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_package_names: &HashSet<String>,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Linking packages to project...");

        for (_package_key, (pkg, store_path)) in stored_packages {
            if direct_package_names.contains(&pkg.name) {
                if let Err(e) = link_package(&project_dir.join("node_modules"), &pkg.name, store_path) {
                    pacm_logger::error(&format!(
                        "Failed to link {}@{}: {}",
                        pkg.name, pkg.version, e
                    ));
                    pacm_logger::debug(
                        &format!("link_package failed for {}@{}", pkg.name, pkg.version),
                        debug,
                    );
                    return Err(PackageManagerError::LinkingFailed(
                        pkg.name.clone(),
                        e.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn link_single_package_to_project(
        &self,
        project_dir: &Path,
        package_name: &str,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        _debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Linking package to project...");

        let project_node_modules = project_dir.join("node_modules");
        if let Some((pkg, store_path)) = stored_packages
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{}@", package_name)))
            .map(|(_, (pkg, store_path))| (pkg, store_path))
        {
            if let Err(e) = link_package(&project_node_modules, &pkg.name, store_path) {
                pacm_logger::error(&format!("Failed to link {}: {}", pkg.name, e));
                return Err(PackageManagerError::LinkingFailed(
                    pkg.name.clone(),
                    e.to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn update_lockfile(
        &self,
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        let mut lockfile = PacmLock::load(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for (_key, (pkg, _)) in stored_packages {
            lockfile.update_dep(
                &pkg.name,
                LockDependency {
                    version: pkg.version.clone(),
                    resolved: pkg.resolved.clone(),
                    integrity: pkg.integrity.clone(),
                },
            );
        }

        lockfile
            .save(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }

    pub fn update_package_json(
        &self,
        project_dir: &Path,
        package_name: &str,
        package_version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) -> Result<()> {
        let mut pkg = read_package_json(project_dir)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        pkg.add_dependency(package_name, package_version, dep_type, save_exact);

        write_package_json(project_dir, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        Ok(())
    }
}
