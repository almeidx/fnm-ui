use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use versi_backend::RemoteVersion;
use versi_core::{ReleaseSchedule, VersionMeta};
use versi_platform::AppPaths;

#[derive(Serialize, Deserialize)]
pub struct DiskCache {
    pub remote_versions: Vec<RemoteVersion>,
    pub release_schedule: Option<ReleaseSchedule>,
    #[serde(default)]
    pub version_metadata: Option<HashMap<String, VersionMeta>>,
    pub cached_at: DateTime<Utc>,
}

impl DiskCache {
    fn load_from_path(path: &Path) -> Option<Self> {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_to_path(&self, path: &Path) {
        if let Ok(data) = serde_json::to_string(self) {
            let _ = std::fs::write(path, data);
        }
    }

    pub fn load() -> Option<Self> {
        let paths = AppPaths::new().ok()?;
        Self::load_from_path(&paths.version_cache_file())
    }

    pub fn save(&self) {
        let Ok(paths) = AppPaths::new() else {
            return;
        };
        let _ = paths.ensure_dirs();
        self.save_to_path(&paths.version_cache_file());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use versi_backend::{NodeVersion, RemoteVersion};
    use versi_core::VersionMeta;

    use super::DiskCache;

    fn sample_cache() -> DiskCache {
        DiskCache {
            remote_versions: vec![RemoteVersion {
                version: NodeVersion::new(22, 10, 0),
                lts_codename: Some("LTS".to_string()),
                is_latest: true,
            }],
            release_schedule: None,
            version_metadata: Some(HashMap::from([(
                "v22.10.0".to_string(),
                VersionMeta {
                    date: "2025-11-01".to_string(),
                    security: false,
                    npm: Some("10.9.0".to_string()),
                    v8: Some("12.0".to_string()),
                    openssl: Some("3.0.0".to_string()),
                },
            )])),
            cached_at: Utc::now(),
        }
    }

    #[test]
    fn save_to_path_and_load_from_path_round_trip() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("versions.json");
        let cache = sample_cache();

        cache.save_to_path(&path);
        let loaded = DiskCache::load_from_path(&path).expect("cache should load");

        assert_eq!(loaded.remote_versions.len(), 1);
        assert_eq!(
            loaded.remote_versions[0].version,
            NodeVersion::new(22, 10, 0)
        );
        let metadata = loaded
            .version_metadata
            .expect("version metadata should be preserved");
        assert_eq!(
            metadata.get("v22.10.0").and_then(|v| v.npm.as_deref()),
            Some("10.9.0")
        );
    }

    #[test]
    fn load_from_path_returns_none_for_invalid_json() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("invalid.json");
        std::fs::write(&path, "{not-valid-json").expect("invalid file should be written");

        assert!(DiskCache::load_from_path(&path).is_none());
    }
}
