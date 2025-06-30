use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::types::CachedPackage;
use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_resolver::{ResolvedPackage, resolve_full_tree, resolve_full_tree_async};

pub struct DependencyResolver {
    client: Arc<reqwest::Client>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            client: Arc::new(
                reqwest::Client::builder()
                    .pool_max_idle_per_host(10)
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new()),
            ),
        }
    }

    pub fn get_client(&self) -> Arc<reqwest::Client> {
        self.client.clone()
    }

    pub async fn resolve_deps(
        &self,
        direct_deps: &[(String, String)],
        use_lockfile: bool,
        _debug: bool,
    ) -> Result<(
        Vec<CachedPackage>,
        Vec<ResolvedPackage>,
        HashSet<String>,
        HashMap<String, ResolvedPackage>,
    )> {
        pacm_logger::status("Resolving dependency tree...");

        let cached_packages = Vec::new();
        let packages_to_download = Vec::new();
        let mut direct_package_names = HashSet::new();
        let mut all_resolved_packages = Vec::new();

        let resolve_tasks: Vec<_> = direct_deps
            .iter()
            .map(|(name, version_or_range)| {
                let client = self.client.clone();
                let name = name.clone();
                let version_or_range = version_or_range.clone();

                async move {
                    let mut seen = HashSet::new();
                    if use_lockfile {
                        resolve_full_tree(&name, &version_or_range, &mut seen).map_err(|e| {
                            PackageManagerError::VersionResolutionFailed(
                                name.clone(),
                                e.to_string(),
                            )
                        })
                    } else {
                        resolve_full_tree_async(client, &name, &version_or_range, &mut seen)
                            .await
                            .map_err(|e| {
                                PackageManagerError::VersionResolutionFailed(
                                    name.clone(),
                                    e.to_string(),
                                )
                            })
                    }
                }
            })
            .collect();

        let resolve_results = join_all(resolve_tasks).await;

        for (i, result) in resolve_results.into_iter().enumerate() {
            let (name, _) = &direct_deps[i];
            direct_package_names.insert(name.clone());

            match result {
                Ok(resolved_tree) => all_resolved_packages.extend(resolved_tree),
                Err(e) => return Err(e),
            }
        }

        let mut unique_packages = HashMap::new();
        for pkg in all_resolved_packages {
            let key = format!("{}@{}", pkg.name, pkg.version);
            unique_packages.insert(key, pkg);
        }

        let resolved_packages_map = unique_packages.clone();

        Ok((
            cached_packages,
            packages_to_download,
            direct_package_names,
            resolved_packages_map,
        ))
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}
