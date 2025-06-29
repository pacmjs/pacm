use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};

use pacm_registry::fetch_package_info;

pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>, // Name => version range
}

/// All available versions from the registry JSON and resolve them
fn resolve_version(
    available_versions: &serde_json::Value,
    range: &str,
    dist_tags: &HashMap<String, String>,
) -> Option<String> {
    // If the range matches a dist-tag, return the version for that tag
    if let Some(tag_version) = dist_tags.get(range) {
        return Some(tag_version.clone());
    }

    let req = VersionReq::parse(range).ok()?;

    // Collect all versions as (Version, String) pairs
    let mut candidates: Vec<(Version, String)> = available_versions
        .as_object()?
        .iter()
        .filter_map(|(v_str, _)| Version::parse(v_str).ok().map(|ver| (ver, v_str.clone())))
        .collect();

    // Sort descending (highest version first)
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    // If the range does not allow pre-releases, filter them out unless explicitly matched
    let allows_prerelease = req.to_string().contains('-');
    let filtered = candidates.into_iter().filter(|(ver, _)| {
        if !req.matches(ver) {
            return false;
        }
        if !ver.pre.is_empty() && !allows_prerelease {
            // Only allow pre-releases if the range explicitly allows them
            return false;
        }
        true
    });

    // Return the highest version that matches
    filtered.map(|(_, v_str)| v_str).next()
}

pub fn resolve_full_tree(
    name: &str,
    version_range: &str,
    seen: &mut HashSet<String>, // for cycles
) -> anyhow::Result<Vec<ResolvedPackage>> {
    let mut resolved = vec![];

    // Get package info from the registry (e.g., from https://registry.npmjs.org/chalk)
    let pkg_data = fetch_package_info(name)?;
    let selected_version = resolve_version(&pkg_data.versions, version_range, &pkg_data.dist_tags)
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve version for {}", name))?;
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
        let sub = resolve_full_tree(&dep_name, &dep_range, seen)?;
        resolved.extend(sub);
    }

    Ok(resolved)
}
