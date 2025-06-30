use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;
use pacm_logger;
use pacm_project::DependencyType;
use pacm_utils::parse_package_spec;

pub struct InstallHandler;

impl InstallHandler {
    pub fn handle_install_all(debug: bool) -> Result<()> {
        println!(
            "{} {}",
            "pacm".bright_cyan().bold(),
            "install".bright_white()
        );
        println!();
        pacm_core::install_all_deps(".", debug)
    }

    pub fn handle_install_packages(
        packages: &[String],
        dev: bool,
        optional: bool,
        peer: bool,
        global: bool,
        save_exact: bool,
        no_save: bool,
        force: bool,
        debug: bool,
    ) -> Result<()> {
        let dep_type = Self::determine_dependency_type(dev, optional, peer);

        if global {
            pacm_logger::error("Global installation is not yet supported");
            return Ok(());
        }

        for pkg in packages {
            let (name, version_range) = parse_package_spec(pkg);
            Self::print_install_header(pkg);
            
            pacm_core::install_single_dep_enhanced(
                ".",
                &name,
                &version_range,
                dep_type,
                save_exact,
                no_save,
                force,
                debug,
            )?;
        }

        Ok(())
    }

    fn determine_dependency_type(dev: bool, optional: bool, peer: bool) -> DependencyType {
        if dev {
            DependencyType::DevDependencies
        } else if optional {
            DependencyType::OptionalDependencies
        } else if peer {
            DependencyType::PeerDependencies
        } else {
            DependencyType::Dependencies
        }
    }

    fn print_install_header(package: &str) {
        println!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "add".bright_white(),
            package.bright_white()
        );
        println!();
    }
}
