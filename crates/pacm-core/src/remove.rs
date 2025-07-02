use std::path::PathBuf;

use pacm_error::{PackageManagerError, Result};
use pacm_lock::PacmLock;
use pacm_logger;
use pacm_project::{read_package_json, write_package_json};

pub struct RemoveManager;

impl RemoveManager {
    pub fn remove_dep(
        &self,
        project_dir: &str,
        name: &str,
        dev_only: bool,
        debug: bool,
    ) -> Result<()> {
        let path = PathBuf::from(project_dir);
        let mut pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        if pkg.has_dependency(name).is_none() {
            pacm_logger::error(&format!("Package '{}' is not installed", name));
            return Ok(());
        }

        pacm_logger::status(&format!("Removing {}...", name));

        if dev_only {
            if let Some(dev_deps) = &mut pkg.dev_dependencies {
                dev_deps.shift_remove(name);
            }
        } else {
            pkg.remove_dependency(name);
        }

        self.remove_from_node_modules(&path, name, debug)?;

        self.update_lockfile_after_removal(&path, name)?;

        write_package_json(&path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        pacm_logger::finish(&format!("removed {}", name));
        Ok(())
    }

    fn remove_from_node_modules(
        &self,
        project_dir: &PathBuf,
        name: &str,
        debug: bool,
    ) -> Result<()> {
        let project_node_modules = project_dir.join("node_modules");
        let package_path = if name.starts_with('@') {
            if let Some(slash_pos) = name.find('/') {
                let scope = &name[..slash_pos];
                let pkg_name = &name[slash_pos + 1..];
                project_node_modules.join(scope).join(pkg_name)
            } else {
                project_node_modules.join(name)
            }
        } else {
            project_node_modules.join(name)
        };

        if package_path.exists() {
            if let Err(e) = std::fs::remove_dir_all(&package_path) {
                pacm_logger::debug(&format!("Failed to remove package directory: {}", e), debug);
                return Err(PackageManagerError::LinkingFailed(
                    name.to_string(),
                    format!("Failed to remove directory: {}", e),
                ));
            }
        }

        Ok(())
    }

    fn update_lockfile_after_removal(&self, project_dir: &PathBuf, name: &str) -> Result<()> {
        let lock_path = project_dir.join("pacm.lock");

        if !lock_path.exists() {
            // No lockfile to update
            return Ok(());
        }

        let mut lockfile = PacmLock::load(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        lockfile.remove_dep_exact(name);

        lockfile
            .save(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }
}
