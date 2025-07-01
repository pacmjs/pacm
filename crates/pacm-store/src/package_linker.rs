use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs, io,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct PackageLinker;

impl PackageLinker {
    pub fn link_packages_batch(
        project_node_modules: &Path,
        packages: &HashMap<String, &Path>,
    ) -> io::Result<()> {
        fs::create_dir_all(project_node_modules)?;

        let counter = AtomicUsize::new(0);

        let results: Result<Vec<_>, _> = packages
            .par_iter()
            .map(|(package_name, store_path)| {
                let result = Self::link_package(project_node_modules, package_name, store_path);

                let current = counter.fetch_add(1, Ordering::Relaxed) + 1;
                if current % 50 == 0 {
                    eprintln!("Linked {}/{} packages", current, packages.len());
                }

                result
            })
            .collect();

        results?;
        Ok(())
    }

    pub fn link_package(
        project_node_modules: &Path,
        package_name: &str,
        store_path: &Path,
    ) -> io::Result<()> {
        let dest = Self::get_package_destination(project_node_modules, package_name);

        Self::ensure_parent_directory_exists(&dest)?;
        Self::remove_existing_package(&dest)?;

        let updated_store_path = match store_path.canonicalize() {
            Ok(canonical_path) => canonical_path.join("package"),
            Err(_) => store_path.join("package"),
        };

        Self::create_symlink(&updated_store_path, &dest)?;
        Ok(())
    }

    fn get_package_destination(
        project_node_modules: &Path,
        package_name: &str,
    ) -> std::path::PathBuf {
        if package_name.starts_with('@') {
            if let Some(slash_pos) = package_name.find('/') {
                let scope = &package_name[..slash_pos]; // @types
                let name = &package_name[slash_pos + 1..]; // node
                let scope_dir = project_node_modules.join(scope);
                scope_dir.join(name)
            } else {
                project_node_modules.join(package_name)
            }
        } else {
            project_node_modules.join(package_name)
        }
    }

    fn ensure_parent_directory_exists(dest: &Path) -> io::Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn remove_existing_package(dest: &Path) -> io::Result<()> {
        if dest.exists() {
            if dest.is_dir() {
                fs::remove_dir_all(dest)?;
            } else {
                fs::remove_file(dest)?;
            }
        }
        Ok(())
    }

    fn create_symlink(source: &Path, dest: &Path) -> io::Result<()> {
        #[cfg(target_family = "unix")]
        std::os::unix::fs::symlink(source, dest)?;

        #[cfg(target_family = "windows")]
        std::os::windows::fs::symlink_dir(source, dest)?;

        Ok(())
    }
}

pub fn link_package(
    project_node_modules: &Path,
    package_name: &str,
    store_path: &Path,
) -> io::Result<()> {
    PackageLinker::link_package(project_node_modules, package_name, store_path)
}
