use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub mod comparators;
pub mod resolver;
pub mod semver;
pub mod version_utils;

pub use resolver::DependencyResolver;

#[derive(Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>, // Name => version range
}

// Backward compatibility functions
pub fn resolve_full_tree(
    name: &str,
    version_range: &str,
    seen: &mut HashSet<String>,
) -> anyhow::Result<Vec<ResolvedPackage>> {
    let resolver = DependencyResolver;
    resolver.resolve_full_tree(name, version_range, seen)
}

pub async fn resolve_full_tree_async(
    client: Arc<reqwest::Client>,
    name: &str,
    version_range: &str,
    seen: &mut HashSet<String>,
) -> anyhow::Result<Vec<ResolvedPackage>> {
    let resolver = DependencyResolver;
    resolver
        .resolve_full_tree_async(client, name, version_range, seen)
        .await
}
