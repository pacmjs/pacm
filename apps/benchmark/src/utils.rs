use std::fs;
use tempfile::TempDir;

pub fn create_temp_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;

    let package_json_content = r#"{
  "name": "benchmark-test-project",
  "version": "1.0.0",
  "description": "Temporary project for benchmarking",
  "main": "index.js",
  "dependencies": {},
  "devDependencies": {}
}"#;

    let package_json_path = temp_dir.path().join("package.json");
    fs::write(package_json_path, package_json_content)?;

    Ok(temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_project() {
        let temp_dir = create_temp_project().unwrap();
        assert!(temp_dir.path().join("package.json").exists());
    }
}
