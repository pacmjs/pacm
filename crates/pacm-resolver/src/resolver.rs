use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ResolvedPackage;
use crate::platform::is_platform_compatible;
use crate::semver::resolve_version;
use pacm_logger;
use pacm_registry::{fetch_package_info, fetch_package_info_async};

pub struct DependencyResolver {
    resolution_cache: Arc<Mutex<HashMap<String, Vec<ResolvedPackage>>>>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            resolution_cache: Arc::new(Mutex::new(HashMap::with_capacity(1000))), // Pre-allocate capacity
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

        let optional_dependencies: HashMap<String, String> = version_data
            .get("optionalDependencies")
            .and_then(|d| d.as_object())
            .map(|deps| {
                deps.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let os = version_data
            .get("os")
            .and_then(|os| os.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

        let cpu = version_data
            .get("cpu")
            .and_then(|cpu| cpu.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

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
            optional_dependencies,
            os,
            cpu,
        };

        resolved.push(resolved_pkg.clone());

        for (dep_name, dep_range) in dependencies {
            let sub = self.resolve_full_tree(&dep_name, &dep_range, seen)?;
            resolved.extend(sub);
        }

        for (dep_name, dep_range) in &resolved_pkg.optional_dependencies {
            match self.resolve_full_tree(dep_name, dep_range, seen) {
                Ok(sub) => {
                    let mut all_compatible = true;
                    for pkg in &sub {
                        if !is_platform_compatible(&pkg.os, &pkg.cpu) {
                            all_compatible = false;
                            // pacm_logger::warn(&format!(
                            //     "Optional dependency {} is not compatible with current platform, skipping",
                            //     pkg.name
                            // ));
                            break;
                        }
                    }

                    if all_compatible {
                        resolved.extend(sub);
                    }
                }
                Err(e) => {
                    pacm_logger::warn(&format!(
                        "Failed to resolve optional dependency {}: {}. Continuing without it.",
                        dep_name, e
                    ));
                }
            }
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

        let mut resolved = Vec::with_capacity(50); // Pre-allocate capacity
        let pkg_data = fetch_package_info_async(client.clone(), name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch package info for {}: {}", name, e))?;

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

        let optional_dependencies: HashMap<String, String> = version_data
            .get("optionalDependencies")
            .and_then(|d| d.as_object())
            .map(|deps| {
                deps.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("*").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let os = version_data
            .get("os")
            .and_then(|os| os.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

        let cpu = version_data
            .get("cpu")
            .and_then(|cpu| cpu.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            });

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
            optional_dependencies,
            os,
            cpu,
        };

        resolved.push(resolved_pkg);

        if !dependencies.is_empty() {
            let dep_tasks: Vec<_> = dependencies
                .into_iter()
                .map(|(dep_name, dep_range)| {
                    let client_clone = client.clone();
                    let resolver = DependencyResolver::new();

                    async move {
                        let mut local_seen = HashSet::with_capacity(100); // Pre-allocate
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

        let current_pkg = resolved.last().unwrap(); // We just pushed it
        if !current_pkg.optional_dependencies.is_empty() {
            let optional_dep_tasks: Vec<_> = current_pkg
                .optional_dependencies
                .iter()
                .map(|(dep_name, dep_range)| {
                    let client_clone = client.clone();
                    let resolver = DependencyResolver::new();
                    let dep_name = dep_name.clone();
                    let dep_range = dep_range.clone();

                    async move {
                        let mut local_seen = HashSet::with_capacity(100);
                        let result = resolver
                            .resolve_full_tree_async(
                                client_clone,
                                &dep_name,
                                &dep_range,
                                &mut local_seen,
                            )
                            .await;

                        (dep_name, result)
                    }
                })
                .collect();

            let optional_results = join_all(optional_dep_tasks).await;

            for (dep_name, dep_result) in optional_results {
                match dep_result {
                    Ok(sub_packages) => {
                        let mut compatible_packages = Vec::new();
                        for pkg in sub_packages {
                            if is_platform_compatible(&pkg.os, &pkg.cpu) {
                                compatible_packages.push(pkg);
                            } else {
                                // pacm_logger::warn(&format!(
                                //     "Optional dependency {} (version {}) is not compatible with current platform, skipping",
                                //     pkg.name, pkg.version
                                // ));
                            }
                        }

                        for pkg in compatible_packages {
                            let pkg_key = format!("{}@{}", pkg.name, pkg.version);
                            if !seen.contains(&pkg_key) {
                                seen.insert(pkg_key);
                                resolved.push(pkg);
                            }
                        }
                    }
                    Err(e) => {
                        pacm_logger::warn(&format!(
                            "Failed to resolve optional dependency {}: {} (continuing installation)",
                            dep_name, e
                        ));
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
