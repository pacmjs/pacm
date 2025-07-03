use std::fs;
use std::path::PathBuf;

use pacm_error::{PackageManagerError, Result};
use pacm_logger;
use pacm_store::get_store_path;

pub struct CleanManager;

impl CleanManager {
    pub fn new() -> Self {
        Self
    }

    pub fn clean_cache(&self, debug: bool) -> Result<()> {
        let store_path = get_store_path();

        if !store_path.exists() {
            pacm_logger::info("No package cache found to clean.");
            return Ok(());
        }

        if debug {
            pacm_logger::debug(&format!("Cleaning cache at: {:?}", store_path), debug);
        }

        pacm_logger::status("Cleaning package cache...");

        // Calculate cache size before cleaning
        let cache_size = self.calculate_directory_size(&store_path)?;

        // Remove the entire store directory
        fs::remove_dir_all(&store_path)
            .map_err(|e| PackageManagerError::IoError(format!("Failed to clean cache: {}", e)))?;

        // Recreate the store directory structure
        fs::create_dir_all(&store_path).map_err(|e| {
            PackageManagerError::IoError(format!("Failed to recreate cache directory: {}", e))
        })?;

        let size_mb = cache_size as f64 / 1024.0 / 1024.0;
        pacm_logger::finish(&format!("Cleaned {:.2} MB of cached packages", size_mb));

        Ok(())
    }

    pub fn clean_node_modules(&self, project_dir: &str, debug: bool) -> Result<()> {
        let project_path = PathBuf::from(project_dir);
        let node_modules_path = project_path.join("node_modules");

        if !node_modules_path.exists() {
            pacm_logger::info("No node_modules directory found to clean.");
            return Ok(());
        }

        if debug {
            pacm_logger::debug(
                &format!("Cleaning node_modules at: {:?}", node_modules_path),
                debug,
            );
        }

        pacm_logger::status("Cleaning node_modules...");

        // Calculate node_modules size before cleaning
        let modules_size = self.calculate_directory_size(&node_modules_path)?;

        // Remove the node_modules directory
        fs::remove_dir_all(&node_modules_path).map_err(|e| {
            PackageManagerError::IoError(format!("Failed to clean node_modules: {}", e))
        })?;

        let size_mb = modules_size as f64 / 1024.0 / 1024.0;
        pacm_logger::finish(&format!("Cleaned {:.2} MB from node_modules", size_mb));

        Ok(())
    }

    fn calculate_directory_size(&self, dir: &PathBuf) -> Result<u64> {
        let mut total_size = 0u64;

        if dir.is_dir() {
            for entry in fs::read_dir(dir).map_err(|e| {
                PackageManagerError::IoError(format!("Failed to read directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    PackageManagerError::IoError(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();

                if path.is_dir() {
                    total_size += self.calculate_directory_size(&path)?;
                } else {
                    let metadata = fs::metadata(&path).map_err(|e| {
                        PackageManagerError::IoError(format!("Failed to read file metadata: {}", e))
                    })?;
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}
