use std::collections::{HashMap, HashSet};
use std::path::Path;

use pacm_error::{PackageManagerError, Result};
use pacm_lock::{LockDependency, PacmLock};
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

    pub fn update_direct_only(
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, std::path::PathBuf)>,
        direct_package_names: &HashSet<String>,
    ) -> Result<()> {
        let mut lockfile = PacmLock::load(lock_path)
            .map_err(|e| PackageManagerError::LockfileError(e.to_string()))?;

        for (_key, (pkg, _)) in stored_packages {
            if direct_package_names.contains(&pkg.name) {
                lockfile.update_dep(
                    &pkg.name,
                    LockDependency {
                        version: pkg.version.clone(),
                        resolved: pkg.resolved.clone(),
                        integrity: pkg.integrity.clone(),
                    },
                );
            }
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
}
