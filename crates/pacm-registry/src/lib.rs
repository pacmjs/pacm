use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn fetch_package_info_async(
    client: Arc<reqwest::Client>,
    name: &str,
) -> anyhow::Result<PackageInfo> {
    let encoded_name = urlencoding::encode(name);
    let url = format!("https://registry.npmjs.org/{}", encoded_name);
    let resp = client.get(&url).send().await?.error_for_status()?;
    let json: Value = resp.json().await?;
    let dist_tags: HashMap<String, String> = serde_json::from_value(json["dist-tags"].clone())?;
    Ok(PackageInfo {
        versions: json["versions"].clone(),
        dist_tags,
    })
}

pub fn fetch_package_info(name: &str) -> anyhow::Result<PackageInfo> {
    let rt = tokio::runtime::Runtime::new()?;
    let client = Arc::new(reqwest::Client::new());
    rt.block_on(fetch_package_info_async(client, name))
}

pub struct PackageInfo {
    pub versions: Value,
    pub dist_tags: HashMap<String, String>,
}
