use std::path::PathBuf;

use crate::install::InstallManager;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_project::read_package_json;

pub struct UpdateManager {
    install_manager: InstallManager,
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            install_manager: InstallManager::new(),
        }
    }

    pub fn update_deps(&self, project_dir: &str, packages: &[String], debug: bool) -> Result<()> {
        let path = PathBuf::from(project_dir);
        let pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        if packages.is_empty() {
            self.update_all_dependencies(&pkg, project_dir, debug)
        } else {
            self.update_specific_packages(&pkg, project_dir, packages, debug)
        }
    }

    fn update_all_dependencies(
        &self,
        pkg: &pacm_project::PackageJson,
        project_dir: &str,
        debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Updating all dependencies...");

        let all_deps = pkg.get_all_dependencies();

        if all_deps.is_empty() {
            pacm_logger::finish("No dependencies to update");
            return Ok(());
        }

        for (name, _current_range) in all_deps {
            pacm_logger::status(&format!("Updating {}...", name));

            if let Some(dep_type) = pkg.has_dependency(&name) {
                if let Err(e) = self.install_manager.install_single(
                    project_dir,
                    &name,
                    "latest",
                    dep_type,
                    false, // save_exact
                    false, // no_save
                    true,  // force
                    debug,
                ) {
                    pacm_logger::error(&format!("Failed to update {}: {}", name, e));
                }
            }
        }

        pacm_logger::finish("All dependencies updated");
        Ok(())
    }

    fn update_specific_packages(
        &self,
        pkg: &pacm_project::PackageJson,
        project_dir: &str,
        packages: &[String],
        debug: bool,
    ) -> Result<()> {
        let mut updated_count = 0;
        let mut failed_count = 0;

        for package in packages {
            pacm_logger::status(&format!("Updating {}...", package));

            if let Some(dep_type) = pkg.has_dependency(package) {
                match self.install_manager.install_single(
                    project_dir,
                    package,
                    "latest",
                    dep_type,
                    false, // save_exact
                    false, // no_save
                    true,  // force - ensures we get the latest version
                    debug,
                ) {
                    Ok(()) => {
                        updated_count += 1;
                        pacm_logger::finish(&format!("Updated {}", package));
                    }
                    Err(e) => {
                        failed_count += 1;
                        pacm_logger::error(&format!("Failed to update {}: {}", package, e));
                    }
                }
            } else {
                failed_count += 1;
                pacm_logger::error(&format!("Package '{}' is not installed", package));
            }
        }

        if failed_count == 0 {
            pacm_logger::finish(&format!("Successfully updated {} packages", updated_count));
        } else {
            pacm_logger::finish(&format!(
                "Updated {} packages, {} failed",
                updated_count, failed_count
            ));
        }

        Ok(())
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}
