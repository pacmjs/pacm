use std::collections::{HashMap, HashSet};
use std::path::Path;

use pacm_error::{PackageManagerError, Result};
use pacm_lock::{LockDependency, LockPackage, PacmLock};
use pacm_resolver::ResolvedPackage;

pub struct LockfileManager;

impl LockfileManager {
    pub fn update_all(
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, std::path::PathBuf)>,
    ) -> Result<()> {
        let mut lockfile = PacmLock::load(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for (_key, (pkg, _)) in stored_packages {
            lockfile.update_package(
                &pkg.name,
                LockPackage {
                    version: pkg.version.clone(),
                    resolved: pkg.resolved.clone(),
                    integrity: pkg.integrity.clone(),
                    dependencies: pkg.dependencies.clone(),
                    optional_dependencies: pkg.optional_dependencies.clone(),
                },
            );
        }

        lockfile
            .save(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }

    pub fn update_direct_only(
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, std::path::PathBuf)>,
        direct_package_names: &HashSet<String>,
    ) -> Result<()> {
        let mut lockfile = PacmLock::load(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for name in direct_package_names {
            if let Some((_key, (pkg, _))) =
                stored_packages.iter().find(|(_, (p, _))| &p.name == name)
            {
                let mut workspace_deps = HashMap::new();
                workspace_deps.insert(pkg.name.clone(), pkg.version.clone());
                lockfile.update_workspace_deps("", &workspace_deps, "dependencies");
            }
        }

        for (_key, (pkg, _)) in stored_packages {
            lockfile.update_package(
                &pkg.name,
                LockPackage {
                    version: pkg.version.clone(),
                    resolved: pkg.resolved.clone(),
                    integrity: pkg.integrity.clone(),
                    dependencies: pkg.dependencies.clone(),
                    optional_dependencies: pkg.optional_dependencies.clone(),
                },
            );
        }

        lockfile
            .save(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }

    pub fn update_from_lockfile_install(
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, std::path::PathBuf)>,
    ) -> Result<()> {
        let mut lockfile = PacmLock::load(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for (_key, (pkg, _)) in stored_packages {
            lockfile.update_package(
                &pkg.name,
                LockPackage {
                    version: pkg.version.clone(),
                    resolved: pkg.resolved.clone(),
                    integrity: pkg.integrity.clone(),
                    dependencies: pkg.dependencies.clone(),
                    optional_dependencies: pkg.optional_dependencies.clone(),
                },
            );
        }

        lockfile
            .save(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        Ok(())
    }

    pub fn load_deps(lock_path: &Path) -> Result<HashMap<String, LockDependency>> {
        if lock_path.exists() {
            let lockfile = PacmLock::load(lock_path)
                .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;
            Ok(lockfile.dependencies)
        } else {
            Ok(HashMap::new())
        }
    }

    pub fn load_packages(lock_path: &Path) -> Result<HashMap<String, LockPackage>> {
        if lock_path.exists() {
            let lockfile = PacmLock::load(lock_path)
                .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;
            Ok(lockfile.packages)
        } else {
            Ok(HashMap::new())
        }
    }
}
