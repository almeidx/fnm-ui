use async_trait::async_trait;
use std::path::PathBuf;

use versi_backend::{
    BackendDetection, BackendError, BackendProvider, BackendUpdate, VersionManager,
};

use crate::backend::NvmBackend;
use crate::client::{NvmClient, NvmEnvironment};
use crate::detection::{NvmVariant, detect_nvm, detect_nvm_environment, install_nvm};
use crate::update::check_for_nvm_update;

pub struct NvmProvider {
    variant: std::sync::Mutex<NvmVariant>,
}

impl Default for NvmProvider {
    fn default() -> Self {
        Self {
            variant: std::sync::Mutex::new(NvmVariant::NotFound),
        }
    }
}

impl NvmProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BackendProvider for NvmProvider {
    fn name(&self) -> &'static str {
        "nvm"
    }

    fn display_name(&self) -> &'static str {
        "nvm (Node Version Manager)"
    }

    fn shell_config_marker(&self) -> &'static str {
        "NVM_DIR"
    }

    fn shell_config_label(&self) -> &'static str {
        "nvm (Node Version Manager)"
    }

    async fn detect(&self) -> BackendDetection {
        let detection = detect_nvm().await;

        *self
            .variant
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = detection.variant.clone();

        let path = detection.nvm_dir.clone().or(detection.nvm_exe.clone());

        BackendDetection {
            found: detection.found,
            path,
            version: detection.version,
            in_path: detection.found,
            data_dir: detection.nvm_dir,
        }
    }

    async fn install_backend(&self) -> Result<(), BackendError> {
        install_nvm()
            .await
            .map_err(|e| BackendError::InstallFailed(e.to_string()))
    }

    async fn check_for_update(
        &self,
        client: &reqwest::Client,
        current_version: &str,
    ) -> Result<Option<BackendUpdate>, BackendError> {
        let variant = self
            .variant
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        check_for_nvm_update(client, current_version, &variant).await
    }

    fn create_manager(&self, detection: &BackendDetection) -> Box<dyn VersionManager> {
        let variant = self
            .variant
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        let nvm_detection = crate::detection::NvmDetection {
            found: detection.found,
            nvm_dir: detection.data_dir.clone(),
            nvm_exe: if variant == NvmVariant::Windows {
                detection.path.clone()
            } else {
                None
            },
            version: detection.version.clone(),
            variant,
        };

        let environment =
            detect_nvm_environment(&nvm_detection).unwrap_or_else(|| NvmEnvironment::Unix {
                nvm_dir: detection
                    .data_dir
                    .clone()
                    .or_else(|| detection.path.clone())
                    .unwrap_or_else(|| PathBuf::from("~/.nvm")),
            });

        let client = NvmClient { environment };

        Box::new(NvmBackend::new(client, detection.version.clone()))
    }

    fn create_manager_for_wsl(
        &self,
        distro: String,
        backend_path: String,
    ) -> Box<dyn VersionManager> {
        let nvm_dir = if backend_path.ends_with("nvm.sh") {
            backend_path
                .strip_suffix("/nvm.sh")
                .unwrap_or(&backend_path)
                .to_string()
        } else {
            backend_path
        };

        let client = NvmClient::wsl(distro, nvm_dir);
        Box::new(NvmBackend::new(client, None))
    }

    fn wsl_search_paths(&self) -> Vec<&'static str> {
        vec!["$HOME/.nvm/nvm.sh"]
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use versi_backend::{BackendDetection, BackendProvider};

    use super::NvmProvider;

    #[test]
    fn provider_metadata_is_stable() {
        let provider = NvmProvider::new();

        assert_eq!(provider.name(), "nvm");
        assert_eq!(provider.display_name(), "nvm (Node Version Manager)");
        assert_eq!(provider.shell_config_marker(), "NVM_DIR");
        assert_eq!(provider.shell_config_label(), "nvm (Node Version Manager)");
    }

    #[test]
    fn create_manager_uses_detection_data_dir_for_unix_fallback() {
        let provider = NvmProvider::new();
        let detection = BackendDetection {
            found: true,
            path: Some(PathBuf::from("/custom/nvm.sh")),
            version: Some("0.40.1".to_string()),
            in_path: true,
            data_dir: Some(PathBuf::from("/custom/.nvm")),
        };

        let manager = provider.create_manager(&detection);
        let info = manager.backend_info();

        assert_eq!(info.path, PathBuf::from("/custom/.nvm/nvm.sh"));
        assert_eq!(info.data_dir, Some(PathBuf::from("/custom/.nvm")));
        assert_eq!(info.version.as_deref(), Some("0.40.1"));
    }

    #[test]
    fn create_wsl_manager_trims_nvm_script_suffix() {
        let provider = NvmProvider::new();

        let manager = provider
            .create_manager_for_wsl("Ubuntu".to_string(), "/home/user/.nvm/nvm.sh".to_string());

        assert_eq!(
            manager.backend_info().path,
            PathBuf::from("/home/user/.nvm/nvm.sh")
        );
        assert_eq!(
            manager.backend_info().data_dir,
            Some(PathBuf::from("/home/user/.nvm"))
        );
    }

    #[test]
    fn wsl_search_paths_contains_default_nvm_script_path() {
        let provider = NvmProvider::new();

        assert_eq!(provider.wsl_search_paths(), vec!["$HOME/.nvm/nvm.sh"]);
    }
}
