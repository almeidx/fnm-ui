//! Remote version fetching, release schedule, and update checks.
//!
//! Handles messages: `RemoteVersionsFetched`, `ReleaseScheduleFetched`,
//! `AppUpdateChecked`, `BackendUpdateChecked`

use std::time::{Duration, Instant};

use log::debug;

use iced::Task;

use versi_core::{check_for_update, fetch_release_schedule, fetch_version_metadata};

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::Versi;
use super::async_helpers::{retry_with_delays, run_with_timeout};

impl Versi {
    pub(super) fn handle_fetch_remote_versions(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.available_versions.loading {
                return Task::none();
            }
            state.available_versions.loading = true;
            state.available_versions.remote_request_seq =
                state.available_versions.remote_request_seq.wrapping_add(1);
            let request_seq = state.available_versions.remote_request_seq;

            let backend = state.backend.clone();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);
            let retry_delays = self.settings.retry_delays_secs.clone();

            return Task::perform(
                async move {
                    retry_with_delays("Remote versions fetch", &retry_delays, || {
                        let backend = backend.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch remote versions",
                                backend.list_remote(),
                                |error| AppError::message(error.to_string()),
                            )
                            .await
                        }
                    })
                    .await
                },
                move |result| Message::RemoteVersionsFetched {
                    request_seq,
                    result,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_remote_versions_fetched(
        &mut self,
        request_seq: u64,
        result: Result<Vec<versi_backend::RemoteVersion>, AppError>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if request_seq != state.available_versions.remote_request_seq {
                debug!(
                    "Ignoring stale remote versions response: request_seq={} current_seq={}",
                    request_seq, state.available_versions.remote_request_seq
                );
                return;
            }

            state.available_versions.loading = false;
            match result {
                Ok(versions) => {
                    state.available_versions.set_versions(versions.clone());
                    state.available_versions.fetched_at = Some(Instant::now());
                    state.available_versions.error = None;
                    state.available_versions.loaded_from_disk = false;

                    // Show badge if any installed major line has a newer version available
                    let env = state.active_environment();
                    let installed_majors: std::collections::HashSet<u32> = env
                        .installed_versions
                        .iter()
                        .map(|v| v.version.major)
                        .collect();
                    let has_update = installed_majors.iter().any(|major| {
                        state
                            .available_versions
                            .latest_by_major
                            .get(major)
                            .is_some_and(|latest| !env.installed_set.contains(&latest.to_string()))
                    });
                    super::platform::set_update_badge(has_update);

                    let schedule = state.available_versions.schedule.clone();
                    let metadata = state.available_versions.metadata.clone();
                    // std::thread::spawn, not tokio — Iced doesn't guarantee a tokio runtime context
                    std::thread::spawn(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: schedule,
                            version_metadata: metadata,
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    state.available_versions.error = Some(error);
                }
            }
        }
    }

    pub(super) fn handle_fetch_release_schedule(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.available_versions.schedule_request_seq = state
                .available_versions
                .schedule_request_seq
                .wrapping_add(1);
            let request_seq = state.available_versions.schedule_request_seq;
            let client = self.http_client.clone();
            let retry_delays = self.settings.retry_delays_secs.clone();

            return Task::perform(
                async move {
                    retry_with_delays("Release schedule fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            fetch_release_schedule(&client)
                                .await
                                .map_err(AppError::message)
                        }
                    })
                    .await
                },
                move |result| Message::ReleaseScheduleFetched {
                    request_seq,
                    result: Box::new(result),
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_release_schedule_fetched(
        &mut self,
        request_seq: u64,
        result: Result<versi_core::ReleaseSchedule, AppError>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if request_seq != state.available_versions.schedule_request_seq {
                debug!(
                    "Ignoring stale release schedule response: request_seq={} current_seq={}",
                    request_seq, state.available_versions.schedule_request_seq
                );
                return;
            }

            match result {
                Ok(schedule) => {
                    state.available_versions.schedule = Some(schedule.clone());
                    state.available_versions.schedule_error = None;

                    let versions = state.available_versions.versions.clone();
                    let metadata = state.available_versions.metadata.clone();
                    // std::thread::spawn, not tokio — Iced doesn't guarantee a tokio runtime context
                    std::thread::spawn(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: Some(schedule),
                            version_metadata: metadata,
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    debug!("Release schedule fetch failed: {error}");
                    state.available_versions.schedule_error = Some(error);
                }
            }
        }
    }

    pub(super) fn handle_fetch_version_metadata(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.available_versions.metadata_request_seq = state
                .available_versions
                .metadata_request_seq
                .wrapping_add(1);
            let request_seq = state.available_versions.metadata_request_seq;
            let client = self.http_client.clone();
            let retry_delays = self.settings.retry_delays_secs.clone();

            return Task::perform(
                async move {
                    retry_with_delays("Version metadata fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            fetch_version_metadata(&client)
                                .await
                                .map_err(AppError::message)
                        }
                    })
                    .await
                },
                move |result| Message::VersionMetadataFetched {
                    request_seq,
                    result: Box::new(result),
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_version_metadata_fetched(
        &mut self,
        request_seq: u64,
        result: Result<std::collections::HashMap<String, versi_core::VersionMeta>, AppError>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if request_seq != state.available_versions.metadata_request_seq {
                debug!(
                    "Ignoring stale version metadata response: request_seq={} current_seq={}",
                    request_seq, state.available_versions.metadata_request_seq
                );
                return;
            }

            match result {
                Ok(metadata) => {
                    state.available_versions.metadata = Some(metadata.clone());

                    let versions = state.available_versions.versions.clone();
                    let schedule = state.available_versions.schedule.clone();
                    std::thread::spawn(move || {
                        let cache = crate::cache::DiskCache {
                            remote_versions: versions,
                            release_schedule: schedule,
                            version_metadata: Some(metadata),
                            cached_at: chrono::Utc::now(),
                        };
                        cache.save();
                    });
                }
                Err(error) => {
                    debug!("Version metadata fetch failed: {error}");
                }
            }
        }
    }

    pub(super) fn handle_check_for_app_update(&mut self) -> Task<Message> {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let client = self.http_client.clone();
        Task::perform(
            async move {
                check_for_update(&client, &current_version)
                    .await
                    .map_err(AppError::from)
            },
            |result| Message::AppUpdateChecked(Box::new(result)),
        )
    }

    pub(super) fn handle_app_update_checked(
        &mut self,
        result: Result<Option<versi_core::AppUpdate>, AppError>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.app_update = update,
                Err(e) => debug!("App update check failed: {e}"),
            }
        }
    }

    pub(super) fn handle_check_for_backend_update(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &self.state
            && let Some(version) = &state.active_environment().backend_version
        {
            let version = version.clone();
            let client = self.http_client.clone();
            let provider = self.provider_for_kind(state.backend_name);
            return Task::perform(
                async move {
                    provider
                        .check_for_update(&client, &version)
                        .await
                        .map_err(AppError::from)
                },
                |result| Message::BackendUpdateChecked(Box::new(result)),
            );
        }
        Task::none()
    }

    pub(super) fn handle_backend_update_checked(
        &mut self,
        result: Result<Option<versi_backend::BackendUpdate>, AppError>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            match result {
                Ok(update) => state.backend_update = update,
                Err(e) => debug!("Backend update check failed: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::AppState;

    fn remote(version: &str, is_latest: bool) -> versi_backend::RemoteVersion {
        versi_backend::RemoteVersion {
            version: version.parse().expect("test version should parse"),
            lts_codename: None,
            is_latest,
        }
    }

    fn sample_schedule() -> versi_core::ReleaseSchedule {
        serde_json::from_value(serde_json::json!({
            "versions": {
                "22": {
                    "start": "2024-04-23",
                    "lts": "2024-10-29",
                    "maintenance": "2026-10-20",
                    "end": "2027-04-30",
                    "codename": "Jod"
                }
            }
        }))
        .expect("sample release schedule should deserialize")
    }

    fn sample_metadata() -> HashMap<String, versi_core::VersionMeta> {
        HashMap::from([(
            "v22.10.0".to_string(),
            versi_core::VersionMeta {
                date: "2026-01-01".to_string(),
                security: true,
                npm: Some("11.0.0".to_string()),
                v8: Some("12.0".to_string()),
                openssl: Some("3.4.0".to_string()),
            },
        )])
    }

    #[test]
    fn remote_versions_fetched_updates_cache_on_success() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.available_versions.loading = true;
            state.available_versions.remote_request_seq = 7;
        }

        app.handle_remote_versions_fetched(
            7,
            Ok(vec![remote("v22.10.0", true), remote("v22.9.0", false)]),
        );

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(!state.available_versions.loading);
        assert!(state.available_versions.error.is_none());
        assert_eq!(state.available_versions.versions.len(), 2);
        assert_eq!(
            state.available_versions.latest_by_major.get(&22),
            Some(&"v22.10.0".parse().expect("version parse"))
        );
        assert!(state.available_versions.fetched_at.is_some());
        assert!(!state.available_versions.loaded_from_disk);
    }

    #[test]
    fn release_schedule_fetched_ignores_stale_request() {
        let mut app = test_app_with_two_environments();
        let baseline = sample_schedule();
        if let AppState::Main(state) = &mut app.state {
            state.available_versions.schedule_request_seq = 3;
            state.available_versions.schedule = Some(baseline.clone());
        }

        app.handle_release_schedule_fetched(2, Ok(sample_schedule()));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(
            state
                .available_versions
                .schedule
                .as_ref()
                .expect("baseline schedule should remain")
                .versions
                .len(),
            baseline.versions.len()
        );
    }

    #[test]
    fn release_schedule_fetched_sets_schedule_and_clears_error() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.available_versions.schedule_request_seq = 5;
            state.available_versions.schedule_error = Some(AppError::message("old error"));
        }

        app.handle_release_schedule_fetched(5, Ok(sample_schedule()));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(state.available_versions.schedule.is_some());
        assert!(state.available_versions.schedule_error.is_none());
    }

    #[test]
    fn version_metadata_fetched_ignores_stale_request() {
        let mut app = test_app_with_two_environments();
        let baseline = sample_metadata();
        if let AppState::Main(state) = &mut app.state {
            state.available_versions.metadata_request_seq = 4;
            state.available_versions.metadata = Some(baseline.clone());
        }

        app.handle_version_metadata_fetched(3, Ok(sample_metadata()));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(
            state
                .available_versions
                .metadata
                .as_ref()
                .expect("baseline metadata should remain")
                .get("v22.10.0")
                .and_then(|meta| meta.npm.as_deref()),
            baseline
                .get("v22.10.0")
                .and_then(|meta| meta.npm.as_deref())
        );
    }

    #[test]
    fn version_metadata_fetched_stores_metadata_on_success() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.available_versions.metadata_request_seq = 8;
            state.available_versions.metadata = None;
        }

        app.handle_version_metadata_fetched(8, Ok(sample_metadata()));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(state.available_versions.metadata.is_some());
    }

    #[test]
    fn app_update_checked_sets_update_on_success() {
        let mut app = test_app_with_two_environments();
        let update = versi_core::AppUpdate {
            current_version: "0.9.0".to_string(),
            latest_version: "0.9.1".to_string(),
            release_url: "https://example.com/release".to_string(),
            release_notes: Some("notes".to_string()),
            download_url: Some("https://example.com/download".to_string()),
            download_size: Some(1234),
        };

        app.handle_app_update_checked(Ok(Some(update.clone())));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(
            state
                .app_update
                .as_ref()
                .map(|value| value.latest_version.as_str()),
            Some("0.9.1")
        );
    }

    #[test]
    fn backend_update_checked_sets_update_on_success() {
        let mut app = test_app_with_two_environments();
        let update = versi_backend::BackendUpdate {
            current_version: "1.0.0".to_string(),
            latest_version: "1.1.0".to_string(),
            release_url: "https://example.com/backend".to_string(),
        };

        app.handle_backend_update_checked(Ok(Some(update.clone())));

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(
            state
                .backend_update
                .as_ref()
                .map(|value| value.latest_version.as_str()),
            Some("1.1.0")
        );
    }
}
