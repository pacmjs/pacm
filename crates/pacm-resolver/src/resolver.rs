use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ResolvedPackage;
use crate::semver::resolve_version;
use pacm_registry::{fetch_package_info, fetch_package_info_async};

pub struct DependencyResolver;

impl DependencyResolver {
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

        let mut dep_tasks = Vec::new();
        for (dep_name, dep_range) in dependencies {
            let client_clone = client.clone();
            let mut seen_clone = seen.clone();
            dep_tasks.push(async move {
                self.resolve_full_tree_async(client_clone, &dep_name, &dep_range, &mut seen_clone)
                    .await
            });
        }

        let dep_results = join_all(dep_tasks).await;
        for dep_result in dep_results {
            match dep_result {
                Ok(sub_packages) => resolved.extend(sub_packages),
                Err(e) => {
                    pacm_logger::debug(&format!("Failed to resolve dependency: {}", e), false)
                }
            }
        }

        Ok(resolved)
    }
}
