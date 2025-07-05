use std::collections::{HashMap, HashSet};
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
        self.remove_with_transitive_deps(project_dir, names, dev_only, debug)
    }

    fn find_transitive_dependencies(
        &self,
        project_dir: &PathBuf,
        packages_to_remove: &[String],
        debug: bool,
    ) -> Result<Vec<String>> {
        let lock_path = project_dir.join("pacm.lock");

        if !lock_path.exists() {
            if debug {
                pacm_logger::debug(
                    "No lockfile found, cannot determine transitive dependencies",
                    debug,
                );
            }
            return Ok(Vec::new());
        }

        let lockfile = PacmLock::load(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        let pkg = read_package_json(project_dir)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let mut remaining_direct_deps = HashSet::new();

        if let Some(deps) = &pkg.dependencies {
            for name in deps.keys() {
                if !packages_to_remove.contains(name) {
                    remaining_direct_deps.insert(name.clone());
                }
            }
        }

        if let Some(dev_deps) = &pkg.dev_dependencies {
            for name in dev_deps.keys() {
                if !packages_to_remove.contains(name) {
                    remaining_direct_deps.insert(name.clone());
                }
            }
        }

        if let Some(peer_deps) = &pkg.peer_dependencies {
            for name in peer_deps.keys() {
                if !packages_to_remove.contains(name) {
                    remaining_direct_deps.insert(name.clone());
                }
            }
        }

        if let Some(opt_deps) = &pkg.optional_dependencies {
            for name in opt_deps.keys() {
                if !packages_to_remove.contains(name) {
                    remaining_direct_deps.insert(name.clone());
                }
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Remaining direct dependencies after removal: {:?}",
                    remaining_direct_deps
                ),
                debug,
            );
        }

        let mut dependency_graph: HashMap<String, HashSet<String>> = HashMap::new();

        if !lockfile.packages.is_empty() {
            for (package_name, lock_package) in &lockfile.packages {
                let mut deps = HashSet::new();

                for dep_name in lock_package.dependencies.keys() {
                    deps.insert(dep_name.clone());
                }

                for dep_name in lock_package.optional_dependencies.keys() {
                    deps.insert(dep_name.clone());
                }

                if debug {
                    pacm_logger::debug(
                        &format!("Package {} has dependencies: {:?}", package_name, deps),
                        debug,
                    );
                }

                dependency_graph.insert(package_name.clone(), deps);
            }
        } else {
            for package_key in lockfile.dependencies.keys() {
                if let Some(at_pos) = package_key.rfind('@') {
                    let package_name = &package_key[..at_pos];

                    let node_modules = project_dir.join("node_modules");
                    let package_dir = if package_name.starts_with('@') {
                        if let Some(slash_pos) = package_name.find('/') {
                            let scope = &package_name[..slash_pos];
                            let name = &package_name[slash_pos + 1..];
                            node_modules.join(scope).join(name)
                        } else {
                            node_modules.join(package_name)
                        }
                    } else {
                        node_modules.join(package_name)
                    };

                    let package_json_path = package_dir.join("package.json");
                    if package_json_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                            if let Ok(pkg_json) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                let mut deps = HashSet::new();

                                if let Some(dependencies) =
                                    pkg_json.get("dependencies").and_then(|d| d.as_object())
                                {
                                    for dep_name in dependencies.keys() {
                                        deps.insert(dep_name.clone());
                                    }
                                }

                                if let Some(opt_dependencies) = pkg_json
                                    .get("optionalDependencies")
                                    .and_then(|d| d.as_object())
                                {
                                    for dep_name in opt_dependencies.keys() {
                                        deps.insert(dep_name.clone());
                                    }
                                }

                                if let Some(peer_dependencies) =
                                    pkg_json.get("peerDependencies").and_then(|d| d.as_object())
                                {
                                    for dep_name in peer_dependencies.keys() {
                                        deps.insert(dep_name.clone());
                                    }
                                }

                                dependency_graph.insert(package_name.to_string(), deps);
                            }
                        }
                    } else if debug {
                        pacm_logger::debug(
                            &format!(
                                "Package.json not found for {}, using lockfile data",
                                package_name
                            ),
                            debug,
                        );
                        dependency_graph.insert(package_name.to_string(), HashSet::new());
                    }
                }
            }
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Built dependency graph with {} packages",
                    dependency_graph.len()
                ),
                debug,
            );
        }

        let mut needed_packages = HashSet::new();
        let mut to_visit = remaining_direct_deps.clone();

        while !to_visit.is_empty() {
            let mut next_visit = HashSet::new();

            for package_name in &to_visit {
                if needed_packages.insert(package_name.clone()) {
                    if let Some(deps) = dependency_graph.get(package_name) {
                        for dep in deps {
                            if !needed_packages.contains(dep) {
                                next_visit.insert(dep.clone());
                            }
                        }
                    }
                }
            }

            to_visit = next_visit;
        }

        if debug {
            pacm_logger::debug(
                &format!(
                    "Found {} packages still needed after removal",
                    needed_packages.len()
                ),
                debug,
            );
        }

        let mut transitive_to_remove = Vec::new();

        if !lockfile.packages.is_empty() {
            for package_name in lockfile.packages.keys() {
                if packages_to_remove.contains(package_name) {
                    continue;
                }

                if !needed_packages.contains(package_name) {
                    let is_direct_dependency = pkg
                        .dependencies
                        .as_ref()
                        .map(|deps| deps.contains_key(package_name))
                        .unwrap_or(false)
                        || pkg
                            .dev_dependencies
                            .as_ref()
                            .map(|deps| deps.contains_key(package_name))
                            .unwrap_or(false)
                        || pkg
                            .peer_dependencies
                            .as_ref()
                            .map(|deps| deps.contains_key(package_name))
                            .unwrap_or(false)
                        || pkg
                            .optional_dependencies
                            .as_ref()
                            .map(|deps| deps.contains_key(package_name))
                            .unwrap_or(false);

                    if !is_direct_dependency {
                        transitive_to_remove.push(package_name.clone());
                    } else if debug {
                        pacm_logger::debug(
                            &format!("Keeping {} as it's still a direct dependency", package_name),
                            debug,
                        );
                    }
                }
            }
        } else {
            for package_key in lockfile.dependencies.keys() {
                if let Some(at_pos) = package_key.rfind('@') {
                    let package_name = &package_key[..at_pos];

                    if packages_to_remove.contains(&package_name.to_string()) {
                        continue;
                    }

                    if !needed_packages.contains(package_name) {
                        let is_direct_dependency = pkg
                            .dependencies
                            .as_ref()
                            .map(|deps| deps.contains_key(package_name))
                            .unwrap_or(false)
                            || pkg
                                .dev_dependencies
                                .as_ref()
                                .map(|deps| deps.contains_key(package_name))
                                .unwrap_or(false)
                            || pkg
                                .peer_dependencies
                                .as_ref()
                                .map(|deps| deps.contains_key(package_name))
                                .unwrap_or(false)
                            || pkg
                                .optional_dependencies
                                .as_ref()
                                .map(|deps| deps.contains_key(package_name))
                                .unwrap_or(false);

                        if !is_direct_dependency {
                            transitive_to_remove.push(package_name.to_string());
                        } else if debug {
                            pacm_logger::debug(
                                &format!(
                                    "Keeping {} as it's still a direct dependency",
                                    package_name
                                ),
                                debug,
                            );
                        }
                    }
                }
            }
        }

        if debug && !transitive_to_remove.is_empty() {
            pacm_logger::debug(
                &format!(
                    "Found {} transitive dependencies to remove: {:?}",
                    transitive_to_remove.len(),
                    transitive_to_remove
                ),
                debug,
            );
        }

        Ok(transitive_to_remove)
    }

    pub fn remove_with_transitive_deps(
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
            if debug {
                pacm_logger::debug(
                    &format!("Checking if package '{}' is installed", name),
                    debug,
                );
                if let Some(deps) = &pkg.dependencies {
                    pacm_logger::debug(&format!("Current dependencies: {:?}", deps), debug);
                }
                if let Some(dev_deps) = &pkg.dev_dependencies {
                    pacm_logger::debug(&format!("Current dev dependencies: {:?}", dev_deps), debug);
                }
            }

            let dependency_type = pkg.has_dependency(name);
            if debug {
                pacm_logger::debug(
                    &format!("has_dependency('{}') returned: {:?}", name, dependency_type),
                    debug,
                );
            }

            if dependency_type.is_some() {
                packages_to_remove.push(name.clone());
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
            if debug {
                pacm_logger::debug("No packages to remove, exiting", debug);
            }
            return Ok(());
        }

        if debug {
            pacm_logger::debug(
                &format!("Packages to remove: {:?}", packages_to_remove),
                debug,
            );
        }

        if debug {
            pacm_logger::debug("Finding transitive dependencies...", debug);
        }
        let transitive_deps =
            match self.find_transitive_dependencies(&path, &packages_to_remove, debug) {
                Ok(deps) => {
                    if debug {
                        pacm_logger::debug(
                            &format!("Found {} transitive dependencies: {:?}", deps.len(), deps),
                            debug,
                        );
                    }
                    deps
                }
                Err(e) => {
                    if debug {
                        pacm_logger::debug(
                            &format!("Error finding transitive dependencies: {}", e),
                            debug,
                        );
                    }
                    return Err(e);
                }
            };

        let mut all_packages_to_remove = packages_to_remove.clone();
        all_packages_to_remove.extend(transitive_deps.clone());

        if packages_to_remove.len() == 1 && transitive_deps.is_empty() {
            pacm_logger::status(&format!("Removing {}...", packages_to_remove[0]));
        } else if transitive_deps.is_empty() {
            pacm_logger::status(&format!(
                "Removing {} packages...",
                packages_to_remove.len()
            ));
        } else {
            pacm_logger::status(&format!(
                "Removing {} packages and {} transitive dependencies...",
                packages_to_remove.len(),
                transitive_deps.len()
            ));
        }

        for name in &packages_to_remove {
            if dev_only {
                if let Some(dev_deps) = &mut pkg.dev_dependencies {
                    dev_deps.shift_remove(name);
                }
            } else {
                pkg.remove_dependency(name);
            }
        }

        for name in &all_packages_to_remove {
            self.remove_from_node_modules(&path, name, debug)?;
        }

        let package_names: Vec<&str> = all_packages_to_remove.iter().map(|s| s.as_str()).collect();
        self.update_lockfile_after_batch_removal(&path, &package_names)?;

        self.cleanup_empty_dependency_sections(&mut pkg);

        write_package_json(&path, &pkg)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        self.cleanup_empty_lockfile(&path)?;
        self.cleanup_empty_node_modules(&path)?;

        if packages_to_remove.len() == 1 && transitive_deps.is_empty() {
            pacm_logger::finish(&format!("removed {}", packages_to_remove[0]));
        } else if transitive_deps.is_empty() {
            pacm_logger::finish(&format!(
                "removed {} packages: {}",
                packages_to_remove.len(),
                packages_to_remove.join(", ")
            ));
        } else {
            pacm_logger::finish(&format!(
                "removed {} packages and {} transitive dependencies",
                packages_to_remove.len(),
                transitive_deps.len()
            ));

            if debug {
                pacm_logger::debug(
                    &format!("Direct packages removed: {}", packages_to_remove.join(", ")),
                    debug,
                );
                pacm_logger::debug(
                    &format!(
                        "Transitive dependencies removed: {}",
                        transitive_deps.join(", ")
                    ),
                    debug,
                );
            }
        }

        Ok(())
    }

    pub fn remove_multiple_deps_direct_only(
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
            pacm_logger::status(&format!(
                "Removing {} (direct only)...",
                packages_to_remove[0]
            ));
        } else {
            pacm_logger::status(&format!(
                "Removing {} packages (direct only)...",
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
            pacm_logger::finish(&format!("removed {} (direct only)", packages_to_remove[0]));
        } else {
            let package_list: Vec<String> =
                packages_to_remove.iter().map(|s| s.to_string()).collect();
            pacm_logger::finish(&format!(
                "removed {} packages (direct only): {}",
                packages_to_remove.len(),
                package_list.join(", ")
            ));
        }

        Ok(())
    }

    pub fn remove_multiple_deps_dry_run(
        &self,
        project_dir: &str,
        names: &[String],
        dev_only: bool,
        direct_only: bool,
        debug: bool,
    ) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }

        let path = PathBuf::from(project_dir);
        let pkg = read_package_json(&path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        let mut packages_to_remove = Vec::new();
        let mut not_installed = Vec::new();

        for name in names {
            if pkg.has_dependency(name).is_some() {
                packages_to_remove.push(name.clone());
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

        let mut transitive_deps = Vec::new();

        if !direct_only {
            transitive_deps =
                self.find_transitive_dependencies(&path, &packages_to_remove, debug)?;
        }

        pacm_logger::status("The following packages would be removed:");

        println!("\n📦 Direct packages ({}):", packages_to_remove.len());
        for package in &packages_to_remove {
            let dep_type = if dev_only {
                "devDependency"
            } else {
                pkg.has_dependency(package)
                    .map(|dt| match dt {
                        pacm_project::DependencyType::Dependencies => "dependency",
                        pacm_project::DependencyType::DevDependencies => "devDependency",
                        pacm_project::DependencyType::PeerDependencies => "peerDependency",
                        pacm_project::DependencyType::OptionalDependencies => "optionalDependency",
                    })
                    .unwrap_or("dependency")
            };
            println!("  - {} ({})", package, dep_type);
        }

        if !transitive_deps.is_empty() {
            println!("\n🔗 Transitive dependencies ({}):", transitive_deps.len());
            for package in &transitive_deps {
                println!("  - {} (no longer needed)", package);
            }
        }

        let total_to_remove = packages_to_remove.len() + transitive_deps.len();

        println!("\n📊 Summary:");
        println!("  - Direct packages: {}", packages_to_remove.len());
        println!("  - Transitive dependencies: {}", transitive_deps.len());
        println!("  - Total packages to remove: {}", total_to_remove);

        if direct_only {
            println!("\n💡 Note: Transitive dependency cleanup is disabled (--direct-only mode)");
        } else if transitive_deps.is_empty() {
            println!("\n✅ No unused transitive dependencies found");
        }

        println!("\nTo actually remove these packages, run the same command without --dry-run");

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

        let is_empty = lockfile.packages.is_empty()
            && lockfile.dependencies.is_empty()
            && lockfile.workspaces.values().all(|workspace| {
                workspace.dependencies.is_empty()
                    && workspace.dev_dependencies.is_empty()
                    && workspace.peer_dependencies.is_empty()
                    && workspace.optional_dependencies.is_empty()
            });

        if is_empty {
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
            lockfile.remove_dep(name);
        }

        lockfile
            .save(&lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }
}
