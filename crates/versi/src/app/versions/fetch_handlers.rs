use std::time::{Duration, Instant};

use iced::Task;
use log::debug;
use tokio_util::sync::CancellationToken;

use versi_core::{fetch_release_schedule, fetch_version_metadata};

use crate::error::AppError;
use crate::message::Message;
use crate::state::AppState;

use super::super::Versi;
use super::super::async_helpers::{retry_with_delays, run_with_timeout};
use super::cache_save::enqueue_cache_save;

pub(super) fn handle_fetch_remote_versions(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        if state.available_versions.loading
            && let Some(cancel_token) = state.available_versions.remote_cancel_token.take()
        {
            cancel_token.cancel();
        }
        state.available_versions.loading = true;
        state.available_versions.remote_request_seq =
            state.available_versions.remote_request_seq.wrapping_add(1);
        let request_seq = state.available_versions.remote_request_seq;
        let cancel_token = CancellationToken::new();
        state.available_versions.remote_cancel_token = Some(cancel_token.clone());

        let backend = state.backend.clone();
        let fetch_timeout = Duration::from_secs(app.settings.fetch_timeout_secs);
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Remote versions fetch"))
                    }
                    result = retry_with_delays("Remote versions fetch", &retry_delays, || {
                        let backend = backend.clone();
                        async move {
                            run_with_timeout(
                                fetch_timeout,
                                "Fetch remote versions",
                                backend.list_remote(),
                                |error| AppError::version_fetch_failed("Remote versions", error.to_string()),
                            )
                            .await
                        }
                    }) => result
                }
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
    app: &mut Versi,
    request_seq: u64,
    result: Result<Vec<versi_backend::RemoteVersion>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if request_seq != state.available_versions.remote_request_seq {
            debug!(
                "Ignoring stale remote versions response: request_seq={} current_seq={}",
                request_seq, state.available_versions.remote_request_seq
            );
            return;
        }

        state.available_versions.loading = false;
        state.available_versions.remote_cancel_token = None;
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
                        .is_some_and(|latest| !env.installed_set.contains(latest))
                });
                super::super::platform::set_update_badge(has_update);

                let schedule = state.available_versions.schedule.clone();
                let metadata = state.available_versions.metadata.clone();
                enqueue_cache_save(crate::cache::DiskCache {
                    remote_versions: versions,
                    release_schedule: schedule,
                    version_metadata: metadata,
                    cached_at: chrono::Utc::now(),
                });
            }
            Err(error) => {
                state.available_versions.error = Some(error);
            }
        }

        state.recompute_banner_stats();
    }
}

pub(super) fn handle_fetch_release_schedule(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        if let Some(cancel_token) = state.available_versions.schedule_cancel_token.take() {
            cancel_token.cancel();
        }
        state.available_versions.schedule_request_seq = state
            .available_versions
            .schedule_request_seq
            .wrapping_add(1);
        let request_seq = state.available_versions.schedule_request_seq;
        let cancel_token = CancellationToken::new();
        state.available_versions.schedule_cancel_token = Some(cancel_token.clone());
        let client = app.http_client.clone();
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Release schedule fetch"))
                    }
                    result = retry_with_delays("Release schedule fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            fetch_release_schedule(&client)
                                .await
                                .map_err(|error| AppError::version_fetch_failed("Release schedule", error))
                        }
                    }) => result
                }
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
    app: &mut Versi,
    request_seq: u64,
    result: Result<versi_core::ReleaseSchedule, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if request_seq != state.available_versions.schedule_request_seq {
            debug!(
                "Ignoring stale release schedule response: request_seq={} current_seq={}",
                request_seq, state.available_versions.schedule_request_seq
            );
            return;
        }

        state.available_versions.schedule_cancel_token = None;
        match result {
            Ok(schedule) => {
                state.available_versions.schedule = Some(schedule.clone());
                state.available_versions.schedule_error = None;

                let versions = state.available_versions.versions.clone();
                let metadata = state.available_versions.metadata.clone();
                enqueue_cache_save(crate::cache::DiskCache {
                    remote_versions: versions,
                    release_schedule: Some(schedule),
                    version_metadata: metadata,
                    cached_at: chrono::Utc::now(),
                });
            }
            Err(error) => {
                debug!("Release schedule fetch failed: {error}");
                state.available_versions.schedule_error = Some(error);
            }
        }

        state.recompute_banner_stats();
    }
}

pub(super) fn handle_fetch_version_metadata(app: &mut Versi) -> Task<Message> {
    if let AppState::Main(state) = &mut app.state {
        if let Some(cancel_token) = state.available_versions.metadata_cancel_token.take() {
            cancel_token.cancel();
        }
        state.available_versions.metadata_request_seq = state
            .available_versions
            .metadata_request_seq
            .wrapping_add(1);
        state.available_versions.metadata_error = None;
        let request_seq = state.available_versions.metadata_request_seq;
        let cancel_token = CancellationToken::new();
        state.available_versions.metadata_cancel_token = Some(cancel_token.clone());
        let client = app.http_client.clone();
        let retry_delays = app.settings.retry_delays_secs.clone();

        return Task::perform(
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {
                        Err(AppError::operation_cancelled("Version metadata fetch"))
                    }
                    result = retry_with_delays("Version metadata fetch", &retry_delays, || {
                        let client = client.clone();
                        async move {
                            fetch_version_metadata(&client)
                                .await
                                .map_err(|error| AppError::version_fetch_failed("Version metadata", error))
                        }
                    }) => result
                }
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
    app: &mut Versi,
    request_seq: u64,
    result: Result<std::collections::HashMap<String, versi_core::VersionMeta>, AppError>,
) {
    if let AppState::Main(state) = &mut app.state {
        if request_seq != state.available_versions.metadata_request_seq {
            debug!(
                "Ignoring stale version metadata response: request_seq={} current_seq={}",
                request_seq, state.available_versions.metadata_request_seq
            );
            return;
        }

        state.available_versions.metadata_cancel_token = None;
        match result {
            Ok(metadata) => {
                state.available_versions.metadata = Some(metadata.clone());
                state.available_versions.metadata_error = None;

                let versions = state.available_versions.versions.clone();
                let schedule = state.available_versions.schedule.clone();
                enqueue_cache_save(crate::cache::DiskCache {
                    remote_versions: versions,
                    release_schedule: schedule,
                    version_metadata: Some(metadata),
                    cached_at: chrono::Utc::now(),
                });
            }
            Err(error) => {
                debug!("Version metadata fetch failed: {error}");
                state.available_versions.metadata_error = Some(error);
            }
        }
    }
}
