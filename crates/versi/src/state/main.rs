use std::collections::{HashMap, HashSet};
use std::time::Instant;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
use versi_backend::{BackendUpdate, NodeVersion, RemoteVersion, VersionManager};
use versi_core::{AppUpdate, ReleaseSchedule, VersionMeta};

use crate::backend_kind::BackendKind;
use crate::error::AppError;
use crate::version_query::search_available_versions;

use super::{
    ContextMenu, EnvironmentState, MainViewKind, Modal, OperationQueue, SettingsModalState, Toast,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchFilter {
    Lts,
    Installed,
    NotInstalled,
    Eol,
    Active,
}

pub struct MainState {
    pub environments: Vec<EnvironmentState>,
    pub active_environment_idx: usize,
    pub available_versions: VersionCache,
    pub operation_queue: OperationQueue,
    pub toasts: Vec<Toast>,
    pub modal: Option<Modal>,
    pub search_query: String,
    pub backend: Box<dyn VersionManager>,
    pub app_update: Option<AppUpdate>,
    pub app_update_state: AppUpdateState,
    pub backend_update: Option<BackendUpdate>,
    pub view: MainViewKind,
    pub settings_state: SettingsModalState,
    pub hovered_version: Option<String>,
    pub backend_name: BackendKind,
    pub detected_backends: Vec<BackendKind>,
    pub refresh_rotation: f32,
    pub active_filters: HashSet<SearchFilter>,
    pub context_menu: Option<ContextMenu>,
    pub cursor_position: iced::Point,
}

#[derive(Debug, Clone, Default)]
pub enum AppUpdateState {
    #[default]
    Idle,
    Downloading {
        downloaded: u64,
        total: u64,
    },
    Extracting,
    Applying,
    RestartRequired,
    Failed(AppError),
}

impl std::fmt::Debug for MainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainState")
            .field("environments", &self.environments)
            .field("active_environment_idx", &self.active_environment_idx)
            .field("available_versions", &self.available_versions)
            .field("operation_queue", &self.operation_queue)
            .field("toasts", &self.toasts)
            .field("modal", &self.modal)
            .field("search_query", &self.search_query)
            .field("backend", &self.backend.name())
            .field("app_update", &self.app_update)
            .field("backend_update", &self.backend_update)
            .field("view", &self.view)
            .field("hovered_version", &self.hovered_version)
            .finish_non_exhaustive()
    }
}

impl MainState {
    pub fn new_with_environments(
        backend: Box<dyn VersionManager>,
        environments: Vec<EnvironmentState>,
        backend_name: BackendKind,
    ) -> Self {
        Self {
            environments,
            active_environment_idx: 0,
            available_versions: VersionCache::new(),
            operation_queue: OperationQueue::new(),
            toasts: Vec::new(),
            modal: None,
            search_query: String::new(),
            backend,
            app_update: None,
            app_update_state: AppUpdateState::default(),
            backend_update: None,
            view: MainViewKind::default(),
            settings_state: SettingsModalState::new(),
            hovered_version: None,
            backend_name,
            detected_backends: Vec::new(),
            refresh_rotation: 0.0,
            active_filters: HashSet::new(),
            context_menu: None,
            cursor_position: iced::Point::ORIGIN,
        }
    }

    pub fn active_environment(&self) -> &EnvironmentState {
        &self.environments[self.active_environment_idx]
    }

    pub fn active_environment_mut(&mut self) -> &mut EnvironmentState {
        &mut self.environments[self.active_environment_idx]
    }

    pub fn add_toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn remove_toast(&mut self, id: usize) {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn next_toast_id(&self) -> usize {
        self.toasts.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }

    pub fn navigable_versions(&self, search_results_limit: usize) -> Vec<String> {
        let env = self.active_environment();
        let mut result = Vec::new();

        if self.search_query.is_empty() {
            for group in &env.version_groups {
                if group.is_expanded {
                    for v in &group.versions {
                        result.push(v.version.to_string());
                    }
                }
            }
        } else {
            let search = search_available_versions(
                &self.available_versions.versions,
                &self.search_query,
                search_results_limit,
                &self.active_filters,
                &env.installed_set,
                self.available_versions.schedule.as_ref(),
            );

            for v in search.versions {
                result.push(v.version.to_string());
            }
        }

        result
    }

    pub fn is_version_installed(&self, version_str: &str) -> bool {
        version_str
            .parse()
            .ok()
            .is_some_and(|version| self.active_environment().installed_set.contains(&version))
    }
}

#[derive(Debug)]
pub struct VersionCache {
    pub versions: Vec<RemoteVersion>,
    pub latest_by_major: HashMap<u32, NodeVersion>,
    pub fetched_at: Option<Instant>,
    pub loading: bool,
    pub remote_request_seq: u64,
    pub remote_cancel_token: Option<CancellationToken>,
    pub error: Option<AppError>,
    pub schedule: Option<ReleaseSchedule>,
    pub schedule_request_seq: u64,
    pub schedule_cancel_token: Option<CancellationToken>,
    pub schedule_error: Option<AppError>,
    pub metadata: Option<HashMap<String, VersionMeta>>,
    pub metadata_error: Option<AppError>,
    pub metadata_request_seq: u64,
    pub metadata_cancel_token: Option<CancellationToken>,
    pub loaded_from_disk: bool,
    pub disk_cached_at: Option<DateTime<Utc>>,
}

impl VersionCache {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            latest_by_major: HashMap::new(),
            fetched_at: None,
            loading: false,
            remote_request_seq: 0,
            remote_cancel_token: None,
            error: None,
            schedule: None,
            schedule_request_seq: 0,
            schedule_cancel_token: None,
            schedule_error: None,
            metadata: None,
            metadata_error: None,
            metadata_request_seq: 0,
            metadata_cancel_token: None,
            loaded_from_disk: false,
            disk_cached_at: None,
        }
    }

    pub fn set_versions(&mut self, versions: Vec<RemoteVersion>) {
        self.versions = versions;
        self.recompute_latest_by_major();
    }

    fn recompute_latest_by_major(&mut self) {
        let mut latest_refs: HashMap<u32, &NodeVersion> = HashMap::new();
        for version in &self.versions {
            latest_refs
                .entry(version.version.major)
                .and_modify(|existing| {
                    if version.version > **existing {
                        *existing = &version.version;
                    }
                })
                .or_insert(&version.version);
        }

        self.latest_by_major.clear();
        self.latest_by_major.reserve(latest_refs.len());
        for (major, version) in latest_refs {
            self.latest_by_major.insert(major, version.clone());
        }
    }

    pub fn network_status(&self) -> NetworkStatus {
        if self.loading {
            return NetworkStatus::Fetching;
        }
        if self.error.is_some() {
            if self.versions.is_empty() {
                return NetworkStatus::Offline;
            }
            return NetworkStatus::Stale;
        }
        NetworkStatus::Online
    }
}

pub enum NetworkStatus {
    Online,
    Fetching,
    Offline,
    Stale,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{MainState, NetworkStatus, SearchFilter, VersionCache};
    use crate::backend_kind::BackendKind;
    use crate::state::EnvironmentState;
    use versi_backend::{NodeVersion, RemoteVersion};
    use versi_platform::EnvironmentId;

    #[test]
    fn set_versions_recomputes_latest_major_versions() {
        let mut cache = VersionCache::new();
        cache.set_versions(vec![
            RemoteVersion {
                version: NodeVersion::new(20, 10, 0),
                lts_codename: Some("Iron".to_string()),
                is_latest: false,
            },
            RemoteVersion {
                version: NodeVersion::new(20, 11, 0),
                lts_codename: Some("Iron".to_string()),
                is_latest: true,
            },
            RemoteVersion {
                version: NodeVersion::new(22, 1, 0),
                lts_codename: None,
                is_latest: true,
            },
            RemoteVersion {
                version: NodeVersion::new(22, 0, 1),
                lts_codename: None,
                is_latest: false,
            },
        ]);

        assert_eq!(
            cache.latest_by_major.get(&20),
            Some(&NodeVersion::new(20, 11, 0))
        );
        assert_eq!(
            cache.latest_by_major.get(&22),
            Some(&NodeVersion::new(22, 1, 0))
        );
    }

    #[test]
    fn network_status_reports_expected_state() {
        let mut cache = VersionCache::new();
        assert!(matches!(cache.network_status(), NetworkStatus::Online));

        cache.loading = true;
        assert!(matches!(cache.network_status(), NetworkStatus::Fetching));

        cache.loading = false;
        cache.error = Some(crate::error::AppError::version_fetch_failed(
            "Remote versions",
            "offline",
        ));
        assert!(matches!(cache.network_status(), NetworkStatus::Offline));

        cache.versions = vec![RemoteVersion {
            version: NodeVersion::new(20, 11, 0),
            lts_codename: Some("Iron".to_string()),
            is_latest: true,
        }];
        assert!(matches!(cache.network_status(), NetworkStatus::Stale));
    }

    fn main_state_with_native_env() -> MainState {
        let provider: std::sync::Arc<dyn versi_backend::BackendProvider> =
            std::sync::Arc::new(versi_fnm::FnmProvider::new());
        let backend = provider.create_manager(&versi_backend::BackendDetection {
            found: true,
            path: Some(PathBuf::from("fnm")),
            version: None,
            in_path: true,
            data_dir: None,
        });
        let mut env = EnvironmentState::new(EnvironmentId::Native, BackendKind::Fnm, None);
        env.loading = false;
        MainState::new_with_environments(backend, vec![env], BackendKind::Fnm)
    }

    fn remote(version: NodeVersion, lts: Option<&str>) -> RemoteVersion {
        RemoteVersion {
            version,
            lts_codename: lts.map(str::to_string),
            is_latest: false,
        }
    }

    fn installed(version: NodeVersion, is_default: bool) -> versi_backend::InstalledVersion {
        versi_backend::InstalledVersion {
            version,
            is_default,
            lts_codename: None,
            install_date: None,
            disk_size: None,
        }
    }

    #[test]
    fn navigable_versions_uses_expanded_groups_without_search() {
        let mut state = main_state_with_native_env();
        state.active_environment_mut().update_versions(vec![
            installed(NodeVersion::new(22, 3, 1), true),
            installed(NodeVersion::new(20, 11, 0), false),
        ]);
        state
            .active_environment_mut()
            .version_groups
            .iter_mut()
            .for_each(|g| g.is_expanded = g.major == 22);

        let navigable = state.navigable_versions(10);

        assert_eq!(navigable, vec!["v22.3.1".to_string()]);
    }

    #[test]
    fn navigable_versions_resolves_alias_queries() {
        let mut state = main_state_with_native_env();
        state.available_versions.set_versions(vec![
            remote(NodeVersion::new(24, 1, 0), None),
            remote(NodeVersion::new(22, 11, 0), Some("Jod")),
            remote(NodeVersion::new(20, 12, 0), Some("Iron")),
        ]);

        state.search_query = "latest".to_string();
        assert_eq!(state.navigable_versions(10), vec!["v24.1.0".to_string()]);

        state.search_query = "lts/iron".to_string();
        assert_eq!(state.navigable_versions(10), vec!["v20.12.0".to_string()]);
    }

    #[test]
    fn is_version_installed_checks_active_environment_versions() {
        let mut state = main_state_with_native_env();
        state.active_environment_mut().installed_versions = vec![versi_backend::InstalledVersion {
            version: NodeVersion::new(20, 11, 0),
            is_default: true,
            lts_codename: Some("Iron".to_string()),
            install_date: None,
            disk_size: None,
        }];
        state
            .active_environment_mut()
            .installed_set
            .insert(NodeVersion::new(20, 11, 0));
        state.active_filters.insert(SearchFilter::Lts);

        assert!(state.is_version_installed("v20.11.0"));
        assert!(!state.is_version_installed("v18.19.1"));
    }
}
