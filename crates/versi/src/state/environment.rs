use std::collections::HashSet;

use versi_backend::{InstalledVersion, NodeVersion, VersionGroup};
use versi_platform::EnvironmentId;

use crate::backend_kind::BackendKind;
use crate::error::AppError;

#[derive(Debug)]
pub struct EnvironmentState {
    pub id: EnvironmentId,
    pub name: String,
    pub installed_versions: Vec<InstalledVersion>,
    pub installed_set: HashSet<String>,
    pub version_groups: Vec<VersionGroup>,
    pub default_version: Option<NodeVersion>,
    pub backend_name: BackendKind,
    pub backend_version: Option<String>,
    pub loading: bool,
    pub error: Option<AppError>,
    pub load_request_seq: u64,
    pub available: bool,
}

impl EnvironmentState {
    pub fn new(
        id: EnvironmentId,
        backend_name: BackendKind,
        backend_version: Option<String>,
    ) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            installed_set: HashSet::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version,
            loading: true,
            error: None,
            load_request_seq: 0,
            available: true,
        }
    }

    pub fn unavailable(id: EnvironmentId, backend_name: BackendKind, reason: &str) -> Self {
        let name = id.display_name();
        Self {
            id,
            name,
            installed_versions: Vec::new(),
            installed_set: HashSet::new(),
            version_groups: Vec::new(),
            default_version: None,
            backend_name,
            backend_version: None,
            loading: false,
            error: Some(AppError::message(reason)),
            load_request_seq: 0,
            available: false,
        }
    }

    pub fn update_versions(&mut self, versions: Vec<InstalledVersion>) {
        self.default_version = versions
            .iter()
            .find(|v| v.is_default)
            .map(|v| v.version.clone());
        self.installed_set = versions.iter().map(|v| v.version.to_string()).collect();
        self.version_groups = VersionGroup::from_versions(versions.clone());
        self.installed_versions = versions;
        self.loading = false;
        self.error = None;
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use versi_backend::NodeVersion;
    use versi_platform::EnvironmentId;

    use super::EnvironmentState;
    use crate::backend_kind::BackendKind;

    fn installed(version: &str, is_default: bool) -> versi_backend::InstalledVersion {
        versi_backend::InstalledVersion {
            version: version.parse().expect("test version should parse"),
            is_default,
            lts_codename: Some("LTS".to_string()),
            install_date: Some(Utc::now()),
            disk_size: Some(1024),
        }
    }

    #[test]
    fn new_environment_state_starts_loading_and_available() {
        let state = EnvironmentState::new(
            EnvironmentId::Native,
            BackendKind::Fnm,
            Some("1.38.0".to_string()),
        );

        assert_eq!(state.id, EnvironmentId::Native);
        assert_eq!(state.backend_name, BackendKind::Fnm);
        assert_eq!(state.backend_version.as_deref(), Some("1.38.0"));
        assert!(state.loading);
        assert!(state.available);
        assert!(state.error.is_none());
        assert!(state.installed_versions.is_empty());
    }

    #[test]
    fn unavailable_state_sets_error_and_availability_flags() {
        let state = EnvironmentState::unavailable(
            EnvironmentId::Native,
            BackendKind::Nvm,
            "backend unavailable",
        );

        assert!(!state.loading);
        assert!(!state.available);
        assert_eq!(state.backend_name, BackendKind::Nvm);
        assert!(matches!(
            state.error,
            Some(crate::error::AppError::Message(ref msg)) if msg == "backend unavailable"
        ));
    }

    #[test]
    fn update_versions_refreshes_collections_and_default() {
        let mut state = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        state.loading = true;
        state.error = Some(crate::error::AppError::message("old error"));

        state.update_versions(vec![
            installed("v20.11.0", true),
            installed("v18.19.1", false),
        ]);

        assert_eq!(state.installed_versions.len(), 2);
        assert_eq!(
            state.default_version,
            Some(NodeVersion {
                major: 20,
                minor: 11,
                patch: 0,
            })
        );
        assert!(state.installed_set.contains("v20.11.0"));
        assert!(state.installed_set.contains("v18.19.1"));
        assert_eq!(state.version_groups.len(), 2);
        assert!(!state.loading);
        assert!(state.error.is_none());
    }
}
