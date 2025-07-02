use anyhow::Result;
use owo_colors::OwoColorize;

use pacm_core;
use pacm_logger;
use pacm_project::DependencyType;
use pacm_utils::parse_pkg_spec;

pub struct InstallHandler;

impl InstallHandler {
    pub fn install_all(debug: bool) -> Result<()> {
        println!(
            "{} {}",
            "pacm".bright_cyan().bold(),
            "install".bright_white()
        );
        println!();
        pacm_core::install_all(".", debug)
    }

    pub fn install_pkgs(
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
        let dep_type = Self::get_dep_type(dev, optional, peer);

        if global {
            pacm_logger::error("Global installation is not yet supported");
            return Ok(());
        }

        if packages.len() == 1 {
            let (name, version_range) = parse_pkg_spec(&packages[0]);
            Self::print_header(&packages[0]);

            pacm_core::install_enhanced(
                ".",
                &name,
                &version_range,
                dep_type,
                save_exact,
                no_save,
                force,
                debug,
            )?;
        } else {
            let parsed_packages: Vec<(String, String)> =
                packages.iter().map(|pkg| parse_pkg_spec(pkg)).collect();

            Self::print_batch_header(packages);

            pacm_core::install_multiple(
                ".",
                &parsed_packages,
                dep_type,
                save_exact,
                no_save,
                force,
                debug,
            )?;
        }

        Ok(())
    }

    fn get_dep_type(dev: bool, optional: bool, peer: bool) -> DependencyType {
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

    fn print_header(package: &str) {
        println!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "install".bright_white(),
            package.bright_white()
        );
        println!();
    }

    fn print_batch_header(packages: &[String]) {
        let package_list = packages.join(" ");
        println!(
            "{} {} {}",
            "pacm".bright_cyan().bold(),
            "install".bright_white(),
            package_list.bright_white()
        );
        println!();
    }
}
