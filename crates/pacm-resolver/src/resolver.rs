use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ResolvedPackage;
use crate::semver::resolve_version;
use pacm_logger;
use pacm_registry::{fetch_package_info, fetch_package_info_async};

pub struct DependencyResolver {
    resolution_cache: Arc<Mutex<HashMap<String, Vec<ResolvedPackage>>>>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            resolution_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn resolve_full_tree(
        &self,
        name: &str,
        version_range: &str,
        seen: &mut HashSet<String>,
    ) -> anyhow::Result<Vec<ResolvedPackage>> {
        let mut resolved = vec![];

        let pkg_data = fetch_package_info(name)?;
        let selected_version =
            resolve_version(&pkg_data.versions, version_range, &pkg_data.dist_tags)
                .map_err(|e| anyhow::anyhow!("Cannot resolve version for {}: {}", name, e))?;
        let version_data = &pkg_data.versions[&selected_version];

        let key = format!("{}@{}", name, selected_version);
        if seen.contains(&key) {
            return Ok(vec![]); // Cycle detected → ignore
        }
        seen.insert(key.clone());

        let dependencies: HashMap<String, String> = version_data
            .get("dependencies")
            .and_then(|d| d.as_object())
            .map(|deps| {
                deps.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let resolved_pkg = ResolvedPackage {
            name: name.to_string(),
            version: selected_version.clone(),
            resolved: version_data["dist"]["tarball"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            integrity: version_data["dist"]["integrity"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            dependencies: dependencies.clone(),
        };

        resolved.push(resolved_pkg);

        for (dep_name, dep_range) in dependencies {
            let sub = self.resolve_full_tree(&dep_name, &dep_range, seen)?;
            resolved.extend(sub);
        }

        Ok(resolved)
    }

    pub async fn resolve_full_tree_async(
        &self,
        client: Arc<reqwest::Client>,
        name: &str,
        version_range: &str,
        seen: &mut HashSet<String>,
    ) -> anyhow::Result<Vec<ResolvedPackage>> {
        let cache_key = format!("{}@{}", name, version_range);
        {
            let cache = self.resolution_cache.lock().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                let filtered: Vec<_> = cached_result
                    .iter()
                    .filter(|pkg| !seen.contains(&format!("{}@{}", pkg.name, pkg.version)))
                    .cloned()
                    .collect();

                if !filtered.is_empty() {
                    for pkg in &filtered {
                        seen.insert(format!("{}@{}", pkg.name, pkg.version));
                    }
                    return Ok(filtered);
                }
            }
        }

        let mut resolved = vec![];

        let pkg_data = fetch_package_info_async(client.clone(), name).await?;
        let selected_version =
            resolve_version(&pkg_data.versions, version_range, &pkg_data.dist_tags)
                .map_err(|e| anyhow::anyhow!("Cannot resolve version for {}: {}", name, e))?;
        let version_data = &pkg_data.versions[&selected_version];

        let key = format!("{}@{}", name, selected_version);
        if seen.contains(&key) {
            return Ok(vec![]); // Cycle detected → ignore
        }
        seen.insert(key.clone());

        let dependencies: HashMap<String, String> = version_data
            .get("dependencies")
            .and_then(|d| d.as_object())
            .map(|deps| {
                deps.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let resolved_pkg = ResolvedPackage {
            name: name.to_string(),
            version: selected_version.clone(),
            resolved: version_data["dist"]["tarball"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            integrity: version_data["dist"]["integrity"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            dependencies: dependencies.clone(),
        };

        resolved.push(resolved_pkg);

        if !dependencies.is_empty() {
            let dep_tasks: Vec<_> = dependencies
                .into_iter()
                .map(|(dep_name, dep_range)| {
                    let client_clone = client.clone();
                    let resolver = DependencyResolver::new();

                    async move {
                        let mut local_seen = HashSet::new();
                        resolver
                            .resolve_full_tree_async(
                                client_clone,
                                &dep_name,
                                &dep_range,
                                &mut local_seen,
                            )
                            .await
                    }
                })
                .collect();

            let dep_results = join_all(dep_tasks).await;

            for dep_result in dep_results {
                match dep_result {
                    Ok(sub_packages) => {
                        for pkg in sub_packages {
                            let pkg_key = format!("{}@{}", pkg.name, pkg.version);
                            if !seen.contains(&pkg_key) {
                                seen.insert(pkg_key);
                                resolved.push(pkg);
                            }
                        }
                    }
                    Err(e) => {
                        pacm_logger::debug(&format!("Failed to resolve dependency: {}", e), false);
                    }
                }
            }
        }

        {
            let mut cache = self.resolution_cache.lock().await;
            cache.insert(cache_key, resolved.clone());
        }

        Ok(resolved)
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}
