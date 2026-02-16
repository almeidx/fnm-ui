use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const INDEX_URL: &str = "https://nodejs.org/dist/index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMeta {
    pub date: String,
    pub security: bool,
    pub npm: Option<String>,
    pub v8: Option<String>,
    pub openssl: Option<String>,
}

#[derive(Deserialize)]
struct RawEntry {
    version: String,
    date: String,
    #[serde(default)]
    security: bool,
    #[serde(default)]
    npm: Option<String>,
    #[serde(default)]
    v8: Option<String>,
    #[serde(default)]
    openssl: Option<String>,
}

pub async fn fetch_version_metadata(
    client: &reqwest::Client,
) -> Result<HashMap<String, VersionMeta>, String> {
    let response = client
        .get(INDEX_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version metadata: {e}"))?;

    let entries: Vec<RawEntry> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse version metadata: {e}"))?;

    let map = entries
        .into_iter()
        .map(|e| {
            (
                e.version,
                VersionMeta {
                    date: e.date,
                    security: e.security,
                    npm: e.npm,
                    v8: e.v8,
                    openssl: e.openssl,
                },
            )
        })
        .collect();

    Ok(map)
}
