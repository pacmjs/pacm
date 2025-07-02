use super::bulk::BulkInstaller;
use super::single::SingleInstaller;
use pacm_error::Result;
use pacm_project::DependencyType;

pub struct InstallManager {
    bulk_installer: BulkInstaller,
    single_installer: SingleInstaller,
}

impl InstallManager {
    pub fn new() -> Self {
        Self {
            bulk_installer: BulkInstaller::new(),
            single_installer: SingleInstaller::new(),
        }
    }

    pub fn install_all(&self, project_dir: &str, debug: bool) -> Result<()> {
        self.bulk_installer.install_all(project_dir, debug)
    }

    pub fn install_single(
        &self,
        project_dir: &str,
        name: &str,
        version_range: &str,
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        self.single_installer.install(
            project_dir,
            name,
            version_range,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        )
    }

    pub fn install_multiple(
        &self,
        project_dir: &str,
        packages: &[(String, String)], // (name, version_range) pairs
        dep_type: DependencyType,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        self.single_installer.install_batch(
            project_dir,
            packages,
            dep_type,
            save_exact,
            no_save,
            force,
            debug,
        )
    }
}

impl Default for InstallManager {
    fn default() -> Self {
        Self::new()
    }
}
