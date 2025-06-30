use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use crate::download::PackageDownloader;
use crate::error::{PackageManagerError, Result};
use crate::linker::PackageLinker;
use pacm_logger;
use pacm_project::{DependencyType, read_package_json};
use pacm_resolver::{ResolvedPackage, resolve_full_tree};

pub struct InstallManager {
    downloader: PackageDownloader,
    linker: PackageLinker,
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
        
        let all_deps = pkg.get_all_dependencies();
        let direct_deps: Vec<(String, String)> = all_deps.into_iter().collect();
        let lock_path = path.join("pacm.lock");
        
        // Resolve all dependencies
        let (all_packages, direct_package_names) = self.resolve_dependencies(&direct_deps, debug)?;
        
        // Download and store packages
        let stored_packages = self.downloader.download_packages(&all_packages, debug)?;
        
        // Link dependencies within the store
        self.linker.link_dependencies_to_store(&stored_packages, debug)?;
        
        // Link direct dependencies to project
        self.linker.link_direct_dependencies_to_project(
            &path,
            &stored_packages,
            &direct_package_names,
            debug,
        )?;
        
        // Run postinstall scripts
        self.run_postinstall_scripts(&stored_packages, debug)?;
        
        // Update lockfile
        self.linker.update_lockfile(&lock_path, &stored_packages)?;
        
        let installed_count = stored_packages.len();
        let final_message = if installed_count == 1 {
            "1 package installed".to_string()
        } else {
            format!("{} packages installed", installed_count)
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

        // Check for existing installation
        if let Some(existing_type) = pkg.has_dependency(name) {
            self.handle_existing_dependency(name, existing_type, dep_type, force)?;
        }

        // Resolve dependencies
        let mut seen = HashSet::new();
        let all_packages = resolve_full_tree(name, version_range, &mut seen)
            .map_err(|e| PackageManagerError::VersionResolutionFailed(name.to_string(), e.to_string()))?;

        // Download and store packages
        let stored_packages = self.downloader.download_packages(&all_packages, debug)?;

        // Link dependencies within the store
        self.linker.link_dependencies_to_store(&stored_packages, debug)?;

        // Link package to project
        self.linker.link_single_package_to_project(&path, name, &stored_packages, debug)?;

        // Update package.json if needed
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

        // Update lockfile
        self.linker.update_lockfile(&lock_path, &stored_packages)?;

        let installed_count = stored_packages.len();
        let main_package = stored_packages
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{}@", name)))
            .map(|(_, (pkg, _))| format!("{}@{}", pkg.name, pkg.version))
            .unwrap_or_else(|| format!("{}@unknown", name));

        let final_message = if installed_count == 1 {
            format!("installed {}", main_package)
        } else {
            format!(
                "installed {} (with {} dependencies)",
                main_package,
                installed_count - 1
            )
        };

        pacm_logger::finish(&final_message);
        Ok(())
    }

    fn resolve_dependencies(
        &self,
        direct_deps: &[(String, String)],
        debug: bool,
    ) -> Result<(Vec<ResolvedPackage>, HashSet<String>)> {
        pacm_logger::status("Resolving dependencies...");

        let mut seen = HashSet::new();
        let mut all_packages = Vec::<ResolvedPackage>::new();
        let mut direct_package_names = HashSet::new();

        for (name, version_range) in direct_deps {
            direct_package_names.insert(name.clone());
            pacm_logger::debug(&format!("Resolving {}@{}", name, version_range), debug);

            let pkgs = resolve_full_tree(name, version_range, &mut seen)
                .map_err(|e| {
                    pacm_logger::error(&format!(
                        "Failed to resolve {}@{}: {}",
                        name, version_range, e
                    ));
                    PackageManagerError::VersionResolutionFailed(name.clone(), e.to_string())
                })?;
            
            all_packages.extend(pkgs);
        }

        Ok((all_packages, direct_package_names))
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

            let status = Command::new("sh")
                .arg("-c")
                .arg(&script)
                .current_dir(&store_path.join("package"))
                .status();
            
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
}

impl Default for InstallManager {
    fn default() -> Self {
        Self::new()
    }
}
