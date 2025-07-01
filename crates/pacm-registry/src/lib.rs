use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref PACKAGE_CACHE: Arc<Mutex<HashMap<String, PackageInfo>>> = Arc::new(Mutex::new(HashMap::with_capacity(5000)));
}

pub async fn fetch_package_info_async(
    client: Arc<reqwest::Client>,
    name: &str,
) -> anyhow::Result<PackageInfo> {
    {
        let cache = PACKAGE_CACHE.lock().await;
        if let Some(cached_info) = cache.get(name) {
            return Ok(cached_info.clone());
        }
    }

    let encoded_name = urlencoding::encode(name);
    let url = format!("https://registry.npmjs.org/{}", encoded_name);

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "pacm/1.0.0")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("Request timeout for {}", name)
            } else if e.is_connect() {
                anyhow::anyhow!("Connection failed for {}: {}", name, e)
            } else if e.is_request() {
                anyhow::anyhow!("Request error for {}: {}", name, e)
            } else {
                anyhow::anyhow!("Network error for {}: {}", name, e)
            }
        })?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("HTTP error for {}: {}", name, e))?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON for {}: {}", name, e))?;
    let dist_tags: HashMap<String, String> = serde_json::from_value(json["dist-tags"].clone())
        .map_err(|e| anyhow::anyhow!("Failed to parse dist-tags for {}: {}", name, e))?;

    let package_info = PackageInfo {
        versions: json["versions"].clone(),
        dist_tags,
    };

    {
        let mut cache = PACKAGE_CACHE.lock().await;
        cache.insert(name.to_string(), package_info.clone());
    }

    Ok(package_info)
}

pub fn fetch_package_info(name: &str) -> anyhow::Result<PackageInfo> {
    let rt = tokio::runtime::Runtime::new()?;
    let client = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new()),
    );
    rt.block_on(fetch_package_info_async(client, name))
}

#[derive(Clone, Debug)]
pub struct PackageInfo {
    pub versions: Value,
    pub dist_tags: HashMap<String, String>,
}
