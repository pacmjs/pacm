use reqwest;
use std::sync::Arc;
use tokio::sync::Semaphore;

use pacm_constants::USER_AGENT;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::ResolvedPackage;

pub struct DownloadClient {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl DownloadClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(25)
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .timeout(std::time::Duration::from_secs(45))
                .connect_timeout(std::time::Duration::from_secs(20))
                .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
                .tcp_nodelay(true)
                .user_agent(USER_AGENT)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            semaphore: Arc::new(Semaphore::new(25)),
        }
    }

    pub fn get_client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn get_semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    pub async fn download_tarball(&self, pkg: &ResolvedPackage, debug: bool) -> Result<Vec<u8>> {
        let _permit = self.semaphore.acquire().await.unwrap();

        if !debug {
            pacm_logger::status(&format!("◦ Downloading {}@{}...", pkg.name, pkg.version));
        }

        match self.client.get(&pkg.resolved).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Err(PackageManagerError::NetworkError(format!(
                        "HTTP {} for {}",
                        resp.status(),
                        pkg.resolved
                    )));
                }

                match resp.bytes().await {
                    Ok(bytes) => {
                        if debug {
                            pacm_logger::debug(
                                &format!(
                                    "Downloaded {}@{} ({} bytes)",
                                    pkg.name,
                                    pkg.version,
                                    bytes.len()
                                ),
                                debug,
                            );
                        }
                        Ok(bytes.to_vec())
                    }
                    Err(e) => {
                        pacm_logger::debug(
                            &format!("Failed to read response bytes for {}: {}", pkg.name, e),
                            debug,
                        );
                        Err(PackageManagerError::NetworkError(e.to_string()))
                    }
                }
            }
            Err(e) => {
                pacm_logger::debug(
                    &format!("Network request failed for {}: {}", pkg.name, e),
                    debug,
                );
                Err(PackageManagerError::NetworkError(e.to_string()))
            }
        }
    }

    pub fn download_tarball_sync(&self, pkg: &ResolvedPackage, debug: bool) -> Result<Vec<u8>> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(PackageManagerError::NetworkError(
                "download_tarball_sync called from async context. Use download_tarball instead."
                    .to_string(),
            ));
        }

        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            PackageManagerError::NetworkError(format!("Failed to create async runtime: {}", e))
        })?;

        rt.block_on(self.download_tarball(pkg, debug))
    }
}
