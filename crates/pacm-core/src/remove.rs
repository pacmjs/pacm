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
        self.remove_multiple_deps(project_dir, &[name.to_string()], dev_only, debug)
    }

    pub fn remove_multiple_deps(
        &self,
        project_dir: &str,
        names: &[String],
        dev_only: bool,
        debug: bool,
    ) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }

        let path = PathBuf::from(project_dir);
        let mut pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let mut packages_to_remove = Vec::new();
        let mut not_installed = Vec::new();

        for name in names {
            if pkg.has_dependency(name).is_some() {
                packages_to_remove.push(name);
            } else {
                not_installed.push(name);
            }
        }

        if !not_installed.is_empty() {
            for name in &not_installed {
                pacm_logger::error(&format!("Package '{}' is not installed", name));
            }
        }

        if packages_to_remove.is_empty() {
            return Ok(());
        }

        if packages_to_remove.len() == 1 {
            pacm_logger::status(&format!("Removing {}...", packages_to_remove[0]));
        } else {
            pacm_logger::status(&format!(
                "Removing {} packages...",
                packages_to_remove.len()
            ));
        }

        for name in &packages_to_remove {
            if dev_only {
                if let Some(dev_deps) = &mut pkg.dev_dependencies {
                    dev_deps.shift_remove(*name);
                }
            } else {
                pkg.remove_dependency(name);
            }
        }

        for name in &packages_to_remove {
            self.remove_from_node_modules(&path, name, debug)?;
        }

        let package_names: Vec<&str> = packages_to_remove.iter().map(|s| s.as_str()).collect();
        self.update_lockfile_after_batch_removal(&path, &package_names)?;

        self.cleanup_empty_dependency_sections(&mut pkg);

        write_package_json(&path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        self.cleanup_empty_lockfile(&path)?;

        self.cleanup_empty_node_modules(&path)?;

        if packages_to_remove.len() == 1 {
            pacm_logger::finish(&format!("removed {}", packages_to_remove[0]));
        } else {
            let package_list: Vec<String> =
                packages_to_remove.iter().map(|s| s.to_string()).collect();
            pacm_logger::finish(&format!(
                "removed {} packages: {}",
                packages_to_remove.len(),
                package_list.join(", ")
            ));
        }

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

    fn cleanup_empty_dependency_sections(&self, pkg: &mut pacm_project::PackageJson) {
        if let Some(deps) = &pkg.dependencies {
            if deps.is_empty() {
                pkg.dependencies = None;
            }
        }

        if let Some(dev_deps) = &pkg.dev_dependencies {
            if dev_deps.is_empty() {
                pkg.dev_dependencies = None;
            }
        }

        if let Some(peer_deps) = &pkg.peer_dependencies {
            if peer_deps.is_empty() {
                pkg.peer_dependencies = None;
            }
        }

        if let Some(opt_deps) = &pkg.optional_dependencies {
            if opt_deps.is_empty() {
                pkg.optional_dependencies = None;
            }
        }
    }

    fn cleanup_empty_lockfile(&self, project_dir: &PathBuf) -> Result<()> {
        let lock_path = project_dir.join("pacm.lock");

        if !lock_path.exists() {
            return Ok(());
        }

        let lockfile = PacmLock::load(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        if lockfile.dependencies.is_empty() {
            if let Err(e) = std::fs::remove_file(&lock_path) {
                pacm_logger::debug(&format!("Failed to remove empty lockfile: {}", e), true);
            } else {
                pacm_logger::debug("Removed empty lockfile", true);
            }
        }

        Ok(())
    }

    fn cleanup_empty_node_modules(&self, project_dir: &PathBuf) -> Result<()> {
        let node_modules = project_dir.join("node_modules");

        if !node_modules.exists() {
            return Ok(());
        }

        match std::fs::read_dir(&node_modules) {
            Ok(entries) => {
                let non_hidden_entries: Vec<_> = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        if let Some(name) = entry.file_name().to_str() {
                            !name.starts_with('.')
                        } else {
                            false
                        }
                    })
                    .collect();

                if non_hidden_entries.is_empty() {
                    if let Err(e) = std::fs::remove_dir_all(&node_modules) {
                        pacm_logger::debug(
                            &format!("Failed to remove empty node_modules: {}", e),
                            true,
                        );
                    } else {
                        pacm_logger::debug("Removed empty node_modules directory", true);
                    }
                }
            }
            Err(e) => {
                pacm_logger::debug(
                    &format!("Failed to read node_modules directory: {}", e),
                    true,
                );
            }
        }

        Ok(())
    }

    fn update_lockfile_after_batch_removal(
        &self,
        project_dir: &PathBuf,
        names: &[&str],
    ) -> Result<()> {
        let lock_path = project_dir.join("pacm.lock");

        if !lock_path.exists() {
            return Ok(());
        }

        let mut lockfile = PacmLock::load(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for name in names {
            lockfile.remove_dep_exact(name);
        }

        lockfile
            .save(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }
}
