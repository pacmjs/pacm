use std::fs;
use std::path::Path;
use crate::package_json::PackageJson;

pub fn read_package_json(project_dir: &Path) -> anyhow::Result<PackageJson> {
    let path = project_dir.join("package.json");
    let content = fs::read_to_string(path)?;
    let parsed: PackageJson = serde_json::from_str(&content)?;
    Ok(parsed)
}

pub fn write_package_json(project_dir: &Path, package_json: &PackageJson) -> anyhow::Result<()> {
    let path = project_dir.join("package.json");
    let content = serde_json::to_string_pretty(package_json)?;
    fs::write(path, content)?;
    Ok(())
}
