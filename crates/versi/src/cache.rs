use std::collections::HashMap;
use std::io::Write;
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
        if let Ok(data) = serde_json::to_vec(self) {
            let _ = write_atomic(path, &data);
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

fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "cache path has no parent")
    })?;

    let file_name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("cache");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let pid = std::process::id();

    let mut tmp_path = None;
    for attempt in 0..16_u8 {
        let candidate = parent.join(format!(".{file_name}.{pid}.{timestamp}.{attempt}.tmp"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(data)?;
                file.sync_all()?;
                tmp_path = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }

    let Some(tmp_path) = tmp_path else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "failed to create unique cache temp file",
        ));
    };

    if let Err(error) = replace_file(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }

    Ok(())
}

fn replace_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };

        let src_utf16: Vec<u16> = src
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let dst_utf16: Vec<u16> = dst
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: both paths are NUL-terminated UTF-16 buffers that live for
        // the duration of the FFI call.
        let moved = unsafe {
            MoveFileExW(
                src_utf16.as_ptr(),
                dst_utf16.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::fs::rename(src, dst)
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

    #[test]
    fn save_to_path_replaces_existing_file_atomically() {
        let temp_dir = tempfile::tempdir().expect("temporary directory should be created");
        let path = temp_dir.path().join("versions.json");
        std::fs::write(&path, "{not-valid-json").expect("invalid file should be written");

        let cache = sample_cache();
        cache.save_to_path(&path);

        let loaded = DiskCache::load_from_path(&path).expect("cache should load after overwrite");
        assert_eq!(loaded.remote_versions.len(), 1);

        let temp_files = std::fs::read_dir(temp_dir.path())
            .expect("read temp dir entries")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".versions.json.")
            })
            .count();
        assert_eq!(temp_files, 0);
    }
}
