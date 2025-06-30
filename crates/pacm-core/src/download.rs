use reqwest;
use std::collections::HashMap;

use crate::error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::ResolvedPackage;
use pacm_store::store_package;

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
                pacm_logger::debug(&format!("Skipping already stored {}", key), debug);
                continue;
            }
            installed.insert(key.clone());

            pacm_logger::progress(
                &format!("Downloading {}", pkg.name),
                current_package,
                total_packages,
            );

            let tarball_bytes = self.download_tarball(pkg, debug)?;
            let store_path = self.store_package(pkg, &tarball_bytes, debug)?;

            stored_packages.insert(key, (pkg.clone(), store_path));
        }

        Ok(stored_packages)
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
