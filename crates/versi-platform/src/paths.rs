use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AppPathsError {
    #[error("Could not determine home directory")]
    HomeDirUnavailable,
    #[error("Could not determine config directory")]
    ConfigDirUnavailable,
    #[error("Could not determine cache directory")]
    CacheDirUnavailable,
    #[error("Could not determine data directory")]
    DataDirUnavailable,
}

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// Build application paths for the current platform.
    ///
    /// # Errors
    /// Returns an error when a required base directory (for example the user
    /// home/config/cache/data directory) cannot be determined.
    pub fn new() -> Result<Self, AppPathsError> {
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().ok_or(AppPathsError::HomeDirUnavailable)?;
            Ok(Self {
                config_dir: home.join("Library/Application Support/versi"),
                cache_dir: home.join("Library/Caches/versi"),
                data_dir: home.join("Library/Application Support/versi"),
            })
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                config_dir: dirs::config_dir()
                    .ok_or(AppPathsError::ConfigDirUnavailable)?
                    .join("versi"),
                cache_dir: dirs::cache_dir()
                    .ok_or(AppPathsError::CacheDirUnavailable)?
                    .join("versi"),
                data_dir: dirs::data_dir()
                    .ok_or(AppPathsError::DataDirUnavailable)?
                    .join("versi"),
            })
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Ok(Self {
                config_dir: dirs::config_dir()
                    .ok_or(AppPathsError::ConfigDirUnavailable)?
                    .join("versi"),
                cache_dir: dirs::cache_dir()
                    .ok_or(AppPathsError::CacheDirUnavailable)?
                    .join("versi"),
                data_dir: dirs::data_dir()
                    .ok_or(AppPathsError::DataDirUnavailable)?
                    .join("versi"),
            })
        }
    }

    #[must_use]
    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    #[must_use]
    pub fn version_cache_file(&self) -> PathBuf {
        self.cache_dir.join("versions.json")
    }

    #[must_use]
    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("debug.log")
    }

    /// Ensure all application directories exist on disk.
    ///
    /// # Errors
    /// Returns an error if any directory cannot be created.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::AppPaths;

    fn test_paths() -> AppPaths {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "versi-platform-paths-test-{}-{}",
            std::process::id(),
            nonce
        ));
        AppPaths {
            config_dir: root.join("config"),
            cache_dir: root.join("cache"),
            data_dir: root.join("data"),
        }
    }

    #[test]
    fn file_paths_use_expected_filenames() {
        let paths = test_paths();

        assert!(
            paths
                .settings_file()
                .ends_with(std::path::Path::new("config").join("settings.json"))
        );
        assert!(
            paths
                .version_cache_file()
                .ends_with(std::path::Path::new("cache").join("versions.json"))
        );
        assert!(
            paths
                .log_file()
                .ends_with(std::path::Path::new("data").join("debug.log"))
        );
    }

    #[test]
    fn ensure_dirs_creates_all_directories() {
        let paths = test_paths();
        let root = paths
            .config_dir
            .parent()
            .expect("config dir should have a parent")
            .to_path_buf();

        paths
            .ensure_dirs()
            .expect("ensure_dirs should create application directories");

        assert!(paths.config_dir.is_dir());
        assert!(paths.cache_dir.is_dir());
        assert!(paths.data_dir.is_dir());

        let _ = std::fs::remove_dir_all(root);
    }
}
