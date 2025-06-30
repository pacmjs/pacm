use std::path::PathBuf;

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_store::store_package;

pub struct PackageStorage;

impl PackageStorage {
    pub fn store(pkg: &ResolvedPackage, tarball_bytes: &[u8], debug: bool) -> Result<PathBuf> {
        match store_package(&pkg.name, &pkg.version, tarball_bytes) {
            Ok(path) => {
                pacm_logger::debug(&format!("Stored {} successfully", pkg.name), debug);
                Ok(path)
            }
            Err(e) => {
                pacm_logger::debug(&format!("Failed to store {}: {}", pkg.name, e), debug);
                Err(PackageManagerError::StorageFailed(
                    pkg.name.clone(),
                    format!("Failed to store package: {}", e),
                ))
            }
        }
    }

    pub fn check_exists(pkg: &ResolvedPackage, debug: bool) -> Result<Option<PathBuf>> {
        use pacm_store::get_store_path;

        let store_base = get_store_path();
        let safe_package_name = if pkg.name.starts_with('@') {
            pkg.name.replace('@', "_at_").replace('/', "_slash_")
        } else {
            pkg.name.to_string()
        };

        let npm_dir = store_base.join("npm");
        if !npm_dir.exists() {
            return Ok(None);
        }

        let package_prefix = format!("{safe_package_name}@{}-", pkg.version);

        match std::fs::read_dir(&npm_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let dir_name = entry.file_name();
                    if let Some(name_str) = dir_name.to_str() {
                        if name_str.starts_with(&package_prefix) {
                            let store_path = entry.path();
                            if store_path.is_dir() {
                                let package_dir = store_path.join("package");
                                if package_dir.exists() {
                                    pacm_logger::debug(
                                        &format!("Found in store: {}", name_str),
                                        debug,
                                    );
                                    return Ok(Some(store_path));
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => return Ok(None),
        }

        Ok(None)
    }
}
