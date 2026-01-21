use std::path::PathBuf;

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().expect("No home directory");
            Self {
                config_dir: home.join("Library/Application Support/fnm-ui"),
                cache_dir: home.join("Library/Caches/fnm-ui"),
                data_dir: home.join("Library/Application Support/fnm-ui"),
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self {
                config_dir: dirs::config_dir()
                    .expect("No config directory")
                    .join("fnm-ui"),
                cache_dir: dirs::cache_dir()
                    .expect("No cache directory")
                    .join("fnm-ui"),
                data_dir: dirs::data_dir().expect("No data directory").join("fnm-ui"),
            }
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            Self {
                config_dir: dirs::config_dir()
                    .expect("No config directory")
                    .join("fnm-ui"),
                cache_dir: dirs::cache_dir()
                    .expect("No cache directory")
                    .join("fnm-ui"),
                data_dir: dirs::data_dir().expect("No data directory").join("fnm-ui"),
            }
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn version_cache_file(&self) -> PathBuf {
        self.cache_dir.join("versions.json")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(())
    }
}

impl Default for AppPaths {
    fn default() -> Self {
        Self::new()
    }
}
