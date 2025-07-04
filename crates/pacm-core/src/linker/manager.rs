use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::install::CachedPackage;
use pacm_error::Result;
use pacm_lock::LockDependency;
use pacm_project::DependencyType;
use pacm_resolver::ResolvedPackage;

use super::cache::CacheLinker;
use super::lockfile::LockfileManager;
use super::project::ProjectLinker;
use super::store::StoreLinker;

pub struct PackageLinker;

impl PackageLinker {
    pub fn link_deps_to_store(
        &self,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        StoreLinker::link_deps_to_store(stored_packages, debug)
    }

    pub fn verify_cached_deps(
        &self,
        cached_packages: &[CachedPackage],
        all_stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        CacheLinker::verify_and_fix_deps(cached_packages, all_stored_packages, debug)
    }

    pub fn link_direct_to_project(
        &self,
        project_dir: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_package_names: &HashSet<String>,
        debug: bool,
    ) -> Result<()> {
        ProjectLinker::link_direct_deps(project_dir, stored_packages, direct_package_names, debug)
    }

    pub fn link_all_to_project(
        &self,
        project_dir: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        ProjectLinker::link_all_deps(project_dir, stored_packages, debug)
    }

    pub fn link_single_to_project(
        &self,
        project_dir: &Path,
        package_name: &str,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        debug: bool,
    ) -> Result<()> {
        ProjectLinker::link_single_pkg(project_dir, package_name, stored_packages, debug)
    }

    pub fn update_lockfile(
        &self,
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        LockfileManager::update_all(lock_path, stored_packages)
    }

    pub fn update_lock_direct(
        &self,
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
        direct_package_names: &HashSet<String>,
    ) -> Result<()> {
        LockfileManager::update_direct_only(lock_path, stored_packages, direct_package_names)
    }

    pub fn update_lock_all(
        &self,
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        LockfileManager::update_all(lock_path, stored_packages)
    }

    pub fn update_lock_from_lockfile_install(
        &self,
        lock_path: &Path,
        stored_packages: &HashMap<String, (ResolvedPackage, PathBuf)>,
    ) -> Result<()> {
        LockfileManager::update_from_lockfile_install(lock_path, stored_packages)
    }

    pub fn update_pkg_json(
        &self,
        project_dir: &Path,
        package_name: &str,
        package_version: &str,
        dep_type: DependencyType,
        save_exact: bool,
    ) -> Result<()> {
        ProjectLinker::update_package_json(
            project_dir,
            package_name,
            package_version,
            dep_type,
            save_exact,
        )
    }

    pub fn load_lock_deps(&self, lock_path: &Path) -> Result<HashMap<String, LockDependency>> {
        LockfileManager::load_deps(lock_path)
    }
}
