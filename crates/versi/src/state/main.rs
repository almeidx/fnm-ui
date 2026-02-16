use std::collections::{HashMap, HashSet};
use std::time::Instant;

use chrono::{DateTime, Utc};
use versi_backend::{BackendUpdate, NodeVersion, RemoteVersion, VersionManager};
use versi_core::{AppUpdate, ReleaseSchedule, VersionMeta};

use crate::backend_kind::BackendKind;
use crate::error::AppError;

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
            let query = &self.search_query;
            let query_lower = query.to_lowercase();
            let versions = &self.available_versions.versions;

            if let Some(resolved) = resolve_alias_query(versions, &query_lower) {
                result.push(resolved.version.to_string());
                return result;
            }

            let mut filtered: Vec<&RemoteVersion> = versions
                .iter()
                .filter(|v| {
                    if query_lower == "lts" {
                        return v.lts_codename.is_some();
                    }
                    let version_str = v.version.to_string();
                    version_str.contains(query.as_str())
                        || v.lts_codename
                            .as_ref()
                            .is_some_and(|c| c.to_lowercase().contains(&query_lower))
                })
                .collect();

            filtered.sort_by(|a, b| b.version.cmp(&a.version));

            let mut latest_by_minor: HashMap<(u32, u32), &RemoteVersion> = HashMap::new();
            for v in &filtered {
                let key = (v.version.major, v.version.minor);
                latest_by_minor
                    .entry(key)
                    .and_modify(|existing| {
                        if v.version.patch > existing.version.patch {
                            *existing = v;
                        }
                    })
                    .or_insert(v);
            }

            let mut available: Vec<&RemoteVersion> = latest_by_minor.into_values().collect();
            available.sort_by(|a, b| b.version.cmp(&a.version));
            available.truncate(search_results_limit);

            for v in available {
                result.push(v.version.to_string());
            }
        }

        result
    }

    pub fn is_version_installed(&self, version_str: &str) -> bool {
        self.active_environment()
            .installed_versions
            .iter()
            .any(|v| v.version.to_string() == version_str)
    }
}

#[derive(Debug)]
pub struct VersionCache {
    pub versions: Vec<RemoteVersion>,
    pub latest_by_major: HashMap<u32, NodeVersion>,
    pub fetched_at: Option<Instant>,
    pub loading: bool,
    pub remote_request_seq: u64,
    pub error: Option<AppError>,
    pub schedule: Option<ReleaseSchedule>,
    pub schedule_request_seq: u64,
    pub schedule_error: Option<AppError>,
    pub metadata: Option<HashMap<String, VersionMeta>>,
    pub metadata_request_seq: u64,
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
            error: None,
            schedule: None,
            schedule_request_seq: 0,
            schedule_error: None,
            metadata: None,
            metadata_request_seq: 0,
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

fn resolve_alias_query<'a>(
    versions: &'a [RemoteVersion],
    query_lower: &str,
) -> Option<&'a RemoteVersion> {
    match query_lower {
        "latest" | "stable" | "current" => versions.iter().max_by_key(|v| &v.version),
        "lts/*" => versions
            .iter()
            .filter(|v| v.lts_codename.is_some())
            .max_by_key(|v| &v.version),
        q if q.starts_with("lts/") => {
            let codename = &q[4..];
            versions
                .iter()
                .filter(|v| {
                    v.lts_codename
                        .as_ref()
                        .is_some_and(|c| c.to_lowercase() == codename)
                })
                .max_by_key(|v| &v.version)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::VersionCache;
    use versi_backend::{NodeVersion, RemoteVersion};

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
}
