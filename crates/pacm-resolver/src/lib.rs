use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub mod comparators;
pub mod platform;
pub mod resolver;
pub mod semver;
pub mod version_utils;

pub use platform::{get_current_cpu, get_current_os, is_platform_compatible};
pub use resolver::DependencyResolver;

#[derive(Clone, Debug)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>, // Name => version range
    pub optional_dependencies: HashMap<String, String>, // Name => version range
    pub os: Option<Vec<String>>,               // OS requirements (e.g., ["win32", "darwin"])
    pub cpu: Option<Vec<String>>,              // CPU requirements (e.g., ["x64", "arm64"])
}

pub fn resolve_full_tree(
    name: &str,
    version_range: &str,
    seen: &mut HashSet<String>,
) -> anyhow::Result<Vec<ResolvedPackage>> {
    let resolver = DependencyResolver::new();
    resolver.resolve_full_tree(name, version_range, seen)
}

pub async fn resolve_full_tree_async(
    client: Arc<reqwest::Client>,
    name: &str,
    version_range: &str,
    seen: &mut HashSet<String>,
) -> anyhow::Result<Vec<ResolvedPackage>> {
    let resolver = DependencyResolver::new();
    resolver
        .resolve_full_tree_async(client, name, version_range, seen)
        .await
}
