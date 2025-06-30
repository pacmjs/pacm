use indexmap::IndexMap;
use std::path::Path;

use pacm_logger;
use pacm_project::PackageJson;
use pacm_error::{PackageManagerError, Result};

pub struct InitManager;

impl InitManager {
    pub fn new() -> Self {
        InitManager
    }

    pub fn init_project(
        &self,
        project_dir: &str,
        name: &str,
        description: Option<&str>,
        version: Option<&str>,
        license: Option<&str>,
    ) -> Result<()> {
        let project_path = Path::new(project_dir);
        let package_json_path = project_path.join("package.json");

        if package_json_path.exists() {
            return Err(PackageManagerError::PackageJsonExists(
                package_json_path.to_string_lossy().into_owned(),
            ));
        }

        pacm_logger::init_logger(false);

        let package_json = PackageJson {
            name: Some(name.to_string()),
            version: Some(version.unwrap_or("1.0.0").to_string()),
            description: description.map(String::from),
            license: license.map(String::from),
            main: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            other: IndexMap::new(),
        };

        package_json
            .save(&package_json_path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        pacm_logger::info(&format!(
            "Initialized new package.json in {} with name '{}'",
            project_dir, name
        ));
        Ok(())
    }
}

pub fn init_project(
    project_dir: &str,
    name: &str,
    description: Option<&str>,
    version: Option<&str>,
    license: Option<&str>,
) -> Result<()> {
    let manager = InitManager::new();
    manager.init_project(project_dir, name, description, version, license)
}
