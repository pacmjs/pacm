use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use pacm_constants::{MAX_ATTEMPTS, USER_AGENT};

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
    let url = format!("https://registry.npmjs.org/{encoded_name}");

    let mut attempts = 0;
    let max_attempts = MAX_ATTEMPTS;

    loop {
        attempts += 1;

        let resp_result = client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", USER_AGENT)
            .send()
            .await;

        let resp = match resp_result {
            Ok(resp) => resp,
            Err(e) => {
                if attempts < max_attempts {
                    let delay = std::cmp::min(1000 * u64::from(attempts), 5000);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                return Err(if e.is_timeout() {
                    anyhow::anyhow!("Request timeout for {} after {} attempts", name, attempts)
                } else if e.is_connect() {
                    anyhow::anyhow!("Connection failed for {}: {}", name, e)
                } else if e.is_request() {
                    anyhow::anyhow!("Request error for {}: {}", name, e)
                } else {
                    anyhow::anyhow!("Network error for {}: {}", name, e)
                });
            }
        };

        let resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                if attempts < max_attempts
                    && (e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
                        || e.status() == Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
                        || e.status() == Some(reqwest::StatusCode::SERVICE_UNAVAILABLE))
                {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        1000 * u64::from(attempts),
                    ))
                    .await;
                    continue;
                }
                return Err(anyhow::anyhow!("HTTP error for {}: {}", name, e));
            }
        };

        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_millis(500 * u64::from(attempts)))
                        .await;
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "Failed to read response text for {}: {}",
                    name,
                    e
                ));
            }
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(e) => {
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_millis(500 * u64::from(attempts)))
                        .await;
                    continue;
                }
                return Err(anyhow::anyhow!(
                    "Failed to parse JSON for {} (response length: {}): {}",
                    name,
                    text.len(),
                    e
                ));
            }
        };

        let dist_tags: HashMap<String, String> = serde_json::from_value(
            json.get("dist-tags")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
        )
        .map_err(|e| anyhow::anyhow!("Failed to parse dist-tags for {}: {}", name, e))?;

        let package_info = PackageInfo {
            versions: json
                .get("versions")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            dist_tags,
        };

        {
            let mut cache = PACKAGE_CACHE.lock().await;
            cache.insert(name.to_string(), package_info.clone());
        }

        return Ok(package_info);
    }
}

pub fn fetch_package_info(name: &str) -> anyhow::Result<PackageInfo> {
    let rt = tokio::runtime::Runtime::new()?;
    let client = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(25)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
            .timeout(std::time::Duration::from_secs(45))
            .connect_timeout(std::time::Duration::from_secs(20))
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .tcp_nodelay(true)
            .user_agent(USER_AGENT)
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
