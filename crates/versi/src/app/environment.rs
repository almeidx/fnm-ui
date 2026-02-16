//! Environment switching, version loading, and search.
//!
//! Handles messages: `EnvironmentSelected`, `EnvironmentLoaded`, `RefreshEnvironment`,
//! `VersionGroupToggled`, `SearchChanged`

use std::time::Duration;

use log::{debug, info, trace};

use iced::Task;
use tokio_util::sync::CancellationToken;

use versi_platform::EnvironmentId;

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, MainViewKind, SearchFilter};

use super::Versi;
use super::async_helpers::run_with_timeout;
use super::init::create_backend_for_environment;

impl Versi {
    pub(super) fn handle_environment_loaded(
        &mut self,
        env_id: &EnvironmentId,
        request_seq: u64,
        result: Result<Vec<versi_backend::InstalledVersion>, AppError>,
    ) -> Task<Message> {
        match &result {
            Ok(versions) => {
                info!(
                    "Environment loaded: {:?} with {} versions",
                    env_id,
                    versions.len()
                );
                for v in versions {
                    trace!(
                        "  Installed version: {} (default={})",
                        v.version, v.is_default
                    );
                }
            }
            Err(error) => {
                info!("Environment load failed for {env_id:?}: {error}");
            }
        }

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| &e.id == env_id)
        {
            if env.load_request_seq != request_seq {
                debug!(
                    "Ignoring stale environment load for {:?}: request_seq={} current_seq={}",
                    env_id, request_seq, env.load_request_seq
                );
                return Task::none();
            }

            env.load_cancel_token = None;

            match result {
                Ok(versions) => env.update_versions(versions),
                Err(error) => {
                    env.loading = false;
                    env.error = Some(error);
                }
            }
        }
        self.update_tray_menu();

        if self.pending_minimize
            && !self.pending_show
            && let Some(id) = self.window_id
        {
            self.pending_minimize = false;
            let hide_task = if super::platform::is_wayland() {
                iced::window::minimize(id, true)
            } else {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            };
            return Task::batch([Task::done(Message::HideDockIcon), hide_task]);
        }

        Task::none()
    }

    pub(super) fn handle_environment_selected(&mut self, idx: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if idx >= state.environments.len() || idx == state.active_environment_idx {
                debug!(
                    "Environment selection ignored: idx={}, current={}",
                    idx, state.active_environment_idx
                );
                return Task::none();
            }

            info!("Switching to environment {idx}");
            state.active_environment_idx = idx;

            let env = &state.environments[idx];
            let env_id = env.id.clone();
            debug!("Selected environment: {env_id:?}");

            let needs_load = env.loading || env.installed_versions.is_empty();
            debug!("Environment needs loading: {needs_load}");

            let env_provider = self
                .providers
                .get(&env.backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());
            self.provider = env_provider.clone();

            let new_backend = create_backend_for_environment(
                &env_id,
                &self.backend_path,
                self.backend_dir.as_ref(),
                &env_provider,
            );
            state.backend = new_backend;
            state.backend_name = env.backend_name;

            state.backend_update = None;

            let in_settings = state.view == MainViewKind::Settings;
            if in_settings {
                state.settings_state.checking_shells = true;
            }

            let load_task = if needs_load {
                info!("Loading versions for environment: {env_id:?}");
                let env = state.active_environment_mut();
                if let Some(token) = env.load_cancel_token.take() {
                    token.cancel();
                }
                env.loading = true;
                env.error = None;
                env.load_request_seq = env.load_request_seq.wrapping_add(1);
                let request_seq = env.load_request_seq;
                let cancel_token = CancellationToken::new();
                env.load_cancel_token = Some(cancel_token.clone());

                let backend = state.backend.clone();
                let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

                Task::perform(
                    async move {
                        debug!("Fetching installed versions for {env_id:?}...");
                        let result = tokio::select! {
                            () = cancel_token.cancelled() => {
                                Err(AppError::operation_cancelled("Loading versions"))
                            }
                            result = run_with_timeout(
                                fetch_timeout,
                                "Loading versions",
                                backend.list_installed(),
                                |error| AppError::environment_load_failed(error.to_string()),
                            ) => result
                        };

                        if let Ok(versions) = &result {
                            debug!(
                                "Environment {:?} loaded: {} versions",
                                env_id,
                                versions.len(),
                            );
                        }
                        (env_id, request_seq, result)
                    },
                    |(env_id, request_seq, result)| Message::EnvironmentLoaded {
                        env_id,
                        request_seq,
                        result,
                    },
                )
            } else {
                Task::none()
            };

            let backend_update_task = self.handle_check_for_backend_update();
            let shell_task = if in_settings {
                self.handle_check_shell_setup()
            } else {
                Task::none()
            };

            return Task::batch([load_task, backend_update_task, shell_task]);
        }
        Task::none()
    }

    pub(super) fn handle_refresh_environment(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            if let Some(token) = env.load_cancel_token.take() {
                token.cancel();
            }
            env.loading = true;
            env.error = None;
            env.load_request_seq = env.load_request_seq.wrapping_add(1);
            let env_id = env.id.clone();
            let request_seq = env.load_request_seq;
            let cancel_token = CancellationToken::new();
            env.load_cancel_token = Some(cancel_token.clone());

            state.refresh_rotation = std::f32::consts::TAU / 40.0;
            let backend = state.backend.clone();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

            return Task::perform(
                async move {
                    let result = tokio::select! {
                        () = cancel_token.cancelled() => {
                            Err(AppError::operation_cancelled("Loading versions"))
                        }
                        result = run_with_timeout(
                            fetch_timeout,
                            "Loading versions",
                            backend.list_installed(),
                            |error| AppError::environment_load_failed(error.to_string()),
                        ) => result
                    };
                    (env_id, request_seq, result)
                },
                |(env_id, request_seq, result)| Message::EnvironmentLoaded {
                    env_id,
                    request_seq,
                    result,
                },
            );
        }
        Task::none()
    }

    pub(super) fn handle_version_group_toggled(&mut self, major: u32) {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            if let Some(group) = env.version_groups.iter_mut().find(|g| g.major == major) {
                group.is_expanded = !group.is_expanded;
            }
        }
    }

    pub(super) fn handle_search_changed(&mut self, query: String) {
        if let AppState::Main(state) = &mut self.state {
            if query.is_empty() {
                state.active_filters.clear();
            }
            state.search_query = query;
        }
    }

    pub(super) fn handle_search_filter_toggled(&mut self, filter: SearchFilter) {
        if let AppState::Main(state) = &mut self.state {
            if state.active_filters.contains(&filter) {
                state.active_filters.remove(&filter);
            } else {
                match filter {
                    SearchFilter::Installed => {
                        state.active_filters.remove(&SearchFilter::NotInstalled);
                    }
                    SearchFilter::NotInstalled => {
                        state.active_filters.remove(&SearchFilter::Installed);
                    }
                    SearchFilter::Eol => {
                        state.active_filters.remove(&SearchFilter::Active);
                    }
                    SearchFilter::Active => {
                        state.active_filters.remove(&SearchFilter::Eol);
                    }
                    SearchFilter::Lts => {}
                }
                state.active_filters.insert(filter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio_util::sync::CancellationToken;

    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::AppState;

    #[test]
    fn search_changed_clears_filters_when_query_becomes_empty() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.active_filters = HashSet::from([SearchFilter::Lts, SearchFilter::Installed]);
            state.search_query = "lts".to_string();
        }

        app.handle_search_changed(String::new());

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(state.active_filters.is_empty());
        assert_eq!(state.search_query, "");
    }

    #[test]
    fn search_filter_toggle_enforces_installed_not_installed_exclusivity() {
        let mut app = test_app_with_two_environments();

        app.handle_search_filter_toggled(SearchFilter::Installed);
        app.handle_search_filter_toggled(SearchFilter::NotInstalled);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(!state.active_filters.contains(&SearchFilter::Installed));
        assert!(state.active_filters.contains(&SearchFilter::NotInstalled));
    }

    #[test]
    fn search_filter_toggle_enforces_eol_active_exclusivity() {
        let mut app = test_app_with_two_environments();

        app.handle_search_filter_toggled(SearchFilter::Active);
        app.handle_search_filter_toggled(SearchFilter::Eol);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(!state.active_filters.contains(&SearchFilter::Active));
        assert!(state.active_filters.contains(&SearchFilter::Eol));
    }

    #[test]
    fn version_group_toggled_flips_target_group_only() {
        let mut app = test_app_with_two_environments();
        if let AppState::Main(state) = &mut app.state {
            state.active_environment_mut().version_groups = vec![
                versi_backend::VersionGroup {
                    major: 22,
                    versions: Vec::new(),
                    is_expanded: true,
                },
                versi_backend::VersionGroup {
                    major: 20,
                    versions: Vec::new(),
                    is_expanded: false,
                },
            ];
        }

        app.handle_version_group_toggled(20);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        let groups = &state.active_environment().version_groups;
        assert!(groups.iter().any(|g| g.major == 20 && g.is_expanded));
        assert!(groups.iter().any(|g| g.major == 22 && g.is_expanded));
    }

    #[test]
    fn refresh_environment_cancels_previous_load_token() {
        let mut app = test_app_with_two_environments();
        let old_token = CancellationToken::new();
        if let AppState::Main(state) = &mut app.state {
            state.active_environment_mut().load_cancel_token = Some(old_token.clone());
        }

        let _ = app.handle_refresh_environment();

        assert!(old_token.is_cancelled());
        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert!(state.active_environment().load_cancel_token.is_some());
    }
}
