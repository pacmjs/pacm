use reqwest;
use std::collections::HashMap;

use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_store::store_package;
use pacm_error::{PackageManagerError, Result};

pub struct PackageDownloader {
    client: reqwest::blocking::Client,
}

impl PackageDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn download_packages(
        &self,
        packages: &[ResolvedPackage],
        debug: bool,
    ) -> Result<HashMap<String, (ResolvedPackage, std::path::PathBuf)>> {
        let mut stored_packages = HashMap::new();
        let mut installed = std::collections::HashSet::new();
        let total_packages = packages.len();

        for (current_package, pkg) in packages.iter().enumerate() {
            let current_package = current_package + 1;
            let key = format!("{}@{}", pkg.name, pkg.version);

            if installed.contains(&key) {
                pacm_logger::debug(&format!("Skipping already processed {}", key), debug);
                continue;
            }
            installed.insert(key.clone());

            let store_path =
                if let Some(existing_path) = self.check_existing_in_store(pkg, debug)? {
                    pacm_logger::debug(&format!("Found {} in store cache", key), debug);
                    existing_path
                } else {
                    pacm_logger::progress(
                        &format!("Downloading {}", pkg.name),
                        current_package,
                        total_packages,
                    );

                    let tarball_bytes = self.download_tarball(pkg, debug)?;
                    self.store_package(pkg, &tarball_bytes, debug)?
                };

            stored_packages.insert(key, (pkg.clone(), store_path));
        }

        Ok(stored_packages)
    }

    fn check_existing_in_store(
        &self,
        pkg: &ResolvedPackage,
        debug: bool,
    ) -> Result<Option<std::path::PathBuf>> {
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
                for entry in entries {
                    if let Ok(entry) = entry {
                        let dir_name = entry.file_name();
                        if let Some(name_str) = dir_name.to_str() {
                            if name_str.starts_with(&package_prefix) {
                                let store_path = entry.path();
                                if store_path.is_dir() && store_path.join("package").exists() {
                                    pacm_logger::debug(
                                        &format!("Found existing store entry: {}", name_str),
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

    fn download_tarball(&self, pkg: &ResolvedPackage, debug: bool) -> Result<Vec<u8>> {
        match self.client.get(&pkg.resolved).send() {
            Ok(resp) => match resp.bytes() {
                Ok(bytes) => Ok(bytes.to_vec()),
                Err(e) => {
                    pacm_logger::debug(
                        &format!(
                            "reqwest::blocking::get({}) succeeded but .bytes() failed: {}",
                            pkg.resolved, e
                        ),
                        debug,
                    );
                    Err(PackageManagerError::DownloadFailed(
                        pkg.name.clone(),
                        pkg.version.clone(),
                    ))
                }
            },
            Err(e) => {
                pacm_logger::debug(
                    &format!("reqwest::blocking::get({}) failed: {}", pkg.resolved, e),
                    debug,
                );
                Err(PackageManagerError::NetworkError(e.to_string()))
            }
        }
    }

    fn store_package(
        &self,
        pkg: &ResolvedPackage,
        tarball_bytes: &[u8],
        debug: bool,
    ) -> Result<std::path::PathBuf> {
        match store_package(&pkg.name, &pkg.version, tarball_bytes) {
            Ok(path) => {
                pacm_logger::debug(
                    &format!("Stored {}@{} in global cache", pkg.name, pkg.version),
                    debug,
                );
                Ok(path)
            }
            Err(e) => {
                pacm_logger::debug(
                    &format!(
                        "store_package failed for {}@{}: {}",
                        pkg.name, pkg.version, e
                    ),
                    debug,
                );
                Err(PackageManagerError::StorageFailed(
                    pkg.name.clone(),
                    pkg.version.clone(),
                ))
            }
        }
    }
}

impl Default for PackageDownloader {
    fn default() -> Self {
        Self::new()
    }
}
