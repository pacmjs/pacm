use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct PackageJson {
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    pub scripts: Option<HashMap<String, String>>,
    pub name: Option<String>,
    pub version: Option<String>,
}

pub fn read_package_json(project_dir: &Path) -> anyhow::Result<PackageJson> {
    let path = project_dir.join("package.json");
    let content = fs::read_to_string(path)?;
    let parsed: PackageJson = serde_json::from_str(&content)?;
    Ok(parsed)
}
