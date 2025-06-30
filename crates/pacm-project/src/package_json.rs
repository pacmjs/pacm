use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<IndexMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<IndexMap<String, String>>,
    #[serde(rename = "devDependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<IndexMap<String, String>>,
    #[serde(rename = "peerDependencies", skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<IndexMap<String, String>>,
    #[serde(
        rename = "optionalDependencies",
        skip_serializing_if = "Option::is_none"
    )]
    pub optional_dependencies: Option<IndexMap<String, String>>,
    // Catch-all for other fields to preserve them
    #[serde(flatten)]
    pub other: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy)]
pub enum DependencyType {
    Dependencies,
    DevDependencies,
    PeerDependencies,
    OptionalDependencies,
}

impl PackageJson {
    pub fn get_all_dependencies(&self) -> HashMap<String, String> {
        let mut all_deps = HashMap::new();

        if let Some(deps) = &self.dependencies {
            all_deps.extend(deps.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(dev_deps) = &self.dev_dependencies {
            all_deps.extend(dev_deps.iter().map(|(k, v)| (k.clone(), v.clone())));
        }

        all_deps
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
