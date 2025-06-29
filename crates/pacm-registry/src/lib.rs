use reqwest::blocking::get;
use serde_json::Value;
use std::collections::HashMap;

/// Fetches package info from the npm registry as JSON
pub fn fetch_package_info(name: &str) -> anyhow::Result<PackageInfo> {
    let url = format!("https://registry.npmjs.org/{}", name);
    let resp = get(&url)?.error_for_status()?;
    let json: Value = resp.json()?;
    let dist_tags: HashMap<String, String> = serde_json::from_value(json["dist-tags"].clone())?;
    Ok(PackageInfo {
        versions: json["versions"].clone(),
        dist_tags,
    })
}

pub struct PackageInfo {
    pub versions: Value,
    pub dist_tags: HashMap<String, String>,
}
