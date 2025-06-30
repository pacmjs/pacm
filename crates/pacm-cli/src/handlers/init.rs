use anyhow::Result;
use inquire::{Select, Text};
use std::env;

use pacm_core;

pub struct InitHandler;

impl InitHandler {
    pub fn handle_init_project(yes: &Option<bool>) -> Result<()> {
        let current_dir = env::current_dir()?;
        let default_name = current_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("my-package");

        let use_defaults = yes.unwrap_or(false);

        let project_name = if use_defaults {
            default_name.to_string()
        } else {
            Text::new("Package name:")
                .with_default(default_name)
                .prompt()?
        };

        let project_description = if use_defaults {
            None
        } else {
            let desc = Text::new("Description (optional):")
                .with_default("")
                .prompt()?;
            if desc.trim().is_empty() {
                None
            } else {
                Some(desc)
            }
        };

        let project_version = if use_defaults {
            "1.0.0".to_string()
        } else {
            Text::new("Version:").with_default("1.0.0").prompt()?
        };

        let project_license = if use_defaults {
            None
        } else {
            let license_options = vec![
                "None",
                "MIT",
                "Apache-2.0",
                "GPL-3.0",
                "BSD-3-Clause",
                "ISC",
                "MPL-2.0",
                "LGPL-3.0",
                "Custom",
            ];

            let selected_license = Select::new("License:", license_options).prompt()?;

            if selected_license == "None" {
                None
            } else if selected_license == "Custom" {
                let custom = Text::new("Custom license:").prompt()?;
                if custom.trim().is_empty() {
                    None
                } else {
                    Some(custom)
                }
            } else {
                Some(selected_license.to_string())
            }
        };

        pacm_core::init_project(
            ".",
            &project_name,
            project_description.as_deref(),
            Some(&project_version),
            project_license.as_deref(),
        )?;

        Ok(())
    }
}
