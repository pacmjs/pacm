use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_project::DependencyType;
use pacm_resolver::ResolvedPackage;
use pacm_store::link_package;

pub struct ProjectLinker;

impl ProjectLinker {
    pub fn link_direct_deps(
        project_dir: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_package_names: &HashSet<String>,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Linking packages to project...");

        let project_node_modules = project_dir.join("node_modules");

        let direct_packages: Vec<_> = stored_packages
            .iter()
            .filter(|(_, (pkg, _))| direct_package_names.contains(&pkg.name))
            .collect();

        let results: Vec<_> = direct_packages
            .par_iter()
            .map(|(_, (pkg, store_path))| {
                if let Err(e) = link_package(&project_node_modules, &pkg.name, store_path) {
                    pacm_logger::error(&format!(
                        "Failed to link {}@{}: {}",
                        pkg.name, pkg.version, e
                    ));
                    if debug {
                        pacm_logger::debug(
                            &format!("link_package failed for {}@{}", pkg.name, pkg.version),
                            debug,
                        );
                    }
                    return Err(PackageManagerError::LinkingFailed(
                        pkg.name.clone(),
                        e.to_string(),
                    ));
                }
                Ok(())
            })
            .collect();

        for result in results {
            result?;
        }

        Ok(())
    }

    pub fn link_all_deps(
        project_dir: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Linking all packages to project (flat node_modules)...");

        let project_node_modules = project_dir.join("node_modules");

        let results: Vec<_> = stored_packages
            .par_iter()
            .map(|(_, (pkg, store_path))| {
                if debug {
                    pacm_logger::debug(
                        &format!("Linking {}@{} to project", pkg.name, pkg.version),
                        debug,
                    );
                }

                if let Err(e) = link_package(&project_node_modules, &pkg.name, store_path) {
                    pacm_logger::error(&format!(
                        "Failed to link {}@{}: {}",
                        pkg.name, pkg.version, e
                    ));
                    if debug {
                        pacm_logger::debug(
                            &format!("link_package failed for {}@{}", pkg.name, pkg.version),
                            debug,
                        );
                    }
                    return Err(PackageManagerError::LinkingFailed(
                        pkg.name.clone(),
                        e.to_string(),
                    ));
                }
                Ok(())
            })
            .collect();

        for result in results {
            result?;
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Successfully linked {} packages to project",
                    stored_packages.len()
                ),
                debug,
            );
        }

        Ok(())
    }

    pub fn link_single_pkg(
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

    pub fn update_package_json(
        project_dir: &Path,
        package_name: &str,
        package_version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) -> Result<()> {
        use pacm_project::{read_package_json, write_package_json};

        let mut pkg = read_package_json(project_dir)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        pkg.add_dependency(package_name, package_version, dep_type, save_exact);

        write_package_json(project_dir, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        Ok(())
    }
}
