//! Remote version fetching, release schedule, and update checks.
//!
//! Handles messages: RemoteVersionsFetched, ReleaseScheduleFetched,
//! AppUpdateChecked, BackendUpdateChecked

use std::time::{Duration, Instant};

use log::debug;

use iced::Task;

use versi_core::{check_for_update, fetch_release_schedule, fetch_version_metadata};

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::Versi;

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
                    let mut last_err = AppError::message("Unknown error");
                    for (attempt, &delay) in retry_delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match tokio::time::timeout(fetch_timeout, backend.list_remote()).await {
                            Err(_) => {
                                last_err = AppError::timeout(
                                    "Fetch remote versions",
                                    fetch_timeout.as_secs(),
                                );
                                debug!("Remote versions fetch attempt {} timed out", attempt + 1,);
                            }
                            Ok(Ok(versions)) => return Ok(versions),
                            Ok(Err(e)) => {
                                last_err = AppError::message(e.to_string());
                                debug!(
                                    "Remote versions fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err,
                                );
                            }
                        }
                    }
                    Err(last_err)
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
                    let mut last_err = AppError::message("Unknown error");
                    for (attempt, &delay) in retry_delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match fetch_release_schedule(&client).await {
                            Ok(schedule) => return Ok(schedule),
                            Err(e) => {
                                last_err = AppError::message(e);
                                debug!(
                                    "Release schedule fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err,
                                );
                            }
                        }
                    }
                    Err(last_err)
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
                    debug!("Release schedule fetch failed: {}", error);
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
                    let mut last_err = AppError::message("Unknown error");
                    for (attempt, &delay) in retry_delays.iter().enumerate() {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                        match fetch_version_metadata(&client).await {
                            Ok(metadata) => return Ok(metadata),
                            Err(e) => {
                                last_err = AppError::message(e);
                                debug!(
                                    "Version metadata fetch attempt {} failed: {}",
                                    attempt + 1,
                                    last_err,
                                );
                            }
                        }
                    }
                    Err(last_err)
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
                    debug!("Version metadata fetch failed: {}", error);
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
                Err(e) => debug!("App update check failed: {}", e),
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
                Err(e) => debug!("Backend update check failed: {}", e),
            }
        }
    }
}
