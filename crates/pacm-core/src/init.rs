use indexmap::IndexMap;
use std::path::Path;

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_project::PackageJson;

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

        pacm_logger::status("Initializing new package...");

        // Create basic scripts
        let mut scripts = IndexMap::new();
        scripts.insert(
            "test".to_string(),
            "echo \"Error: no test specified\" && exit 1".to_string(),
        );
        scripts.insert("start".to_string(), "node index.js".to_string());
        scripts.insert(
            "build".to_string(),
            "echo \"No build script specified\"".to_string(),
        );

        let package_json = PackageJson {
            name: Some(name.to_string()),
            version: Some(version.unwrap_or("1.0.0").to_string()),
            description: description
                .map(String::from)
                .or_else(|| Some("".to_string())),
            license: license
                .map(String::from)
                .or_else(|| Some("ISC".to_string())),
            main: Some("index.js".to_string()),
            scripts: Some(scripts),
            dependencies: Some(IndexMap::new()),
            dev_dependencies: Some(IndexMap::new()),
            peer_dependencies: None,
            optional_dependencies: None,
            other: {
                let mut other = IndexMap::new();
                other.insert("keywords".to_string(), serde_json::Value::Array(vec![]));
                other.insert(
                    "author".to_string(),
                    serde_json::Value::String("".to_string()),
                );
                other
            },
        };

        package_json
            .save(&package_json_path)
            .map_err(|e| PackageManagerError::PackageJsonError(e.to_string()))?;

        // Create basic project structure
        self.create_basic_files(project_path)?;

        pacm_logger::finish(&format!(
            "Initialized new package '{}' in {}",
            name, project_dir
        ));

        // Show next steps
        self.show_next_steps(name)?;

        Ok(())
    }

    pub fn init_interactive(&self, project_dir: &str, yes: bool) -> Result<()> {
        if yes {
            // Non-interactive mode with defaults
            let project_path = Path::new(project_dir);
            let dir_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("my-package");

            return self.init_project(
                project_dir,
                dir_name,
                Some("A new package"),
                Some("1.0.0"),
                Some("ISC"),
            );
        }

        // In a real implementation, this would use a proper interactive prompt library
        // For now, we'll use defaults
        pacm_logger::info(
            "Interactive initialization not fully implemented yet. Using defaults...",
        );

        let project_path = Path::new(project_dir);
        let dir_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-package");

        self.init_project(
            project_dir,
            dir_name,
            Some("A new package"),
            Some("1.0.0"),
            Some("ISC"),
        )
    }

    fn create_basic_files(&self, project_path: &Path) -> Result<()> {
        // Create a basic index.js file
        let index_js_path = project_path.join("index.js");
        if !index_js_path.exists() {
            std::fs::write(&index_js_path, "console.log('Hello, world!');\n").map_err(|e| {
                PackageManagerError::IoError(format!("Failed to create index.js: {}", e))
            })?;
        }

        // Create a basic README.md
        let readme_path = project_path.join("README.md");
        if !readme_path.exists() {
            let readme_content = format!(
                "# {}\n\nA new Node.js package.\n\n## Installation\n\n```bash\nnpm install\n```\n\n## Usage\n\n```bash\nnpm start\n```\n",
                project_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("my-package")
            );
            std::fs::write(&readme_path, readme_content).map_err(|e| {
                PackageManagerError::IoError(format!("Failed to create README.md: {}", e))
            })?;
        }

        // Create a .gitignore file
        let gitignore_path = project_path.join(".gitignore");
        if !gitignore_path.exists() {
            let gitignore_content = "node_modules/\n.env\n.DS_Store\ndist/\nbuild/\n*.log\n";
            std::fs::write(&gitignore_path, gitignore_content).map_err(|e| {
                PackageManagerError::IoError(format!("Failed to create .gitignore: {}", e))
            })?;
        }

        Ok(())
    }

    fn show_next_steps(&self, _package_name: &str) -> Result<()> {
        use owo_colors::OwoColorize;

        println!();
        println!("{}", "Next steps:".bold().green());
        println!(
            "  {} Install dependencies: {}",
            "1.".cyan(),
            "pacm install".yellow()
        );
        println!("  {} Start developing: {}", "2.".cyan(), "code .".yellow());
        println!(
            "  {} Run your package: {}",
            "3.".cyan(),
            "pacm start".yellow()
        );
        println!();
        println!("Happy coding! 🚀");
        println!();

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
