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

    pub fn update_dependencies(
        &self,
        project_dir: &str,
        packages: &[String],
        debug: bool,
    ) -> Result<()> {
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
        _project_dir: &str,
        _debug: bool,
    ) -> Result<()> {
        pacm_logger::status("Updating all dependencies...");

        let all_deps = pkg.get_all_dependencies();
        for (name, _) in all_deps {
            pacm_logger::info(&format!("Checking updates for {}...", name));
            // TODO: Implement logic to check for updates
        }

        Ok(())
    }

    fn update_specific_packages(
        &self,
        pkg: &pacm_project::PackageJson,
        project_dir: &str,
        packages: &[String],
        debug: bool,
    ) -> Result<()> {
        for package in packages {
            pacm_logger::status(&format!("Updating {}...", package));

            if let Some(dep_type) = pkg.has_dependency(package) {
                self.install_manager.install_single(
                    project_dir,
                    package,
                    "latest",
                    dep_type,
                    false,
                    false,
                    true,
                    debug,
                )?;
            } else {
                pacm_logger::error(&format!("Package '{}' is not installed", package));
            }
        }

        Ok(())
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}
