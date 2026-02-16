//! Environment switching, version loading, and search.
//!
//! Handles messages: EnvironmentSelected, EnvironmentLoaded, RefreshEnvironment,
//! VersionGroupToggled, SearchChanged

use std::time::Duration;

use log::{debug, info, trace};

use iced::Task;

use versi_platform::EnvironmentId;

use crate::error::AppError;
use crate::message::Message;
use crate::state::{AppState, MainViewKind, SearchFilter};

use super::Versi;
use super::init::create_backend_for_environment;

impl Versi {
    pub(super) fn handle_environment_loaded(
        &mut self,
        env_id: EnvironmentId,
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
                info!("Environment load failed for {:?}: {}", env_id, error);
            }
        }

        if let AppState::Main(state) = &mut self.state
            && let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id)
        {
            match result {
                Ok(versions) => env.update_versions(versions),
                Err(error) => {
                    env.loading = false;
                    env.error = Some(error.to_string());
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

            info!("Switching to environment {}", idx);
            state.active_environment_idx = idx;

            let env = &state.environments[idx];
            let env_id = env.id.clone();
            debug!("Selected environment: {:?}", env_id);

            let needs_load = env.loading || env.installed_versions.is_empty();
            debug!("Environment needs loading: {}", needs_load);

            let env_provider = self
                .providers
                .get(&env.backend_name)
                .cloned()
                .unwrap_or_else(|| self.provider.clone());
            self.provider = env_provider.clone();

            let new_backend = create_backend_for_environment(
                &env_id,
                &self.backend_path,
                &self.backend_dir,
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
                info!("Loading versions for environment: {:?}", env_id);
                let env = state.active_environment_mut();
                env.loading = true;

                let backend = state.backend.clone();
                let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

                Task::perform(
                    async move {
                        debug!("Fetching installed versions for {:?}...", env_id);
                        let result =
                            match tokio::time::timeout(fetch_timeout, backend.list_installed())
                                .await
                            {
                                Ok(Ok(versions)) => Ok(versions),
                                Ok(Err(error)) => {
                                    Err(AppError::message(format!("Failed to load versions: {error}")))
                                }
                                Err(_) => {
                                    Err(AppError::timeout("Loading versions", fetch_timeout.as_secs()))
                                }
                            };

                        if let Ok(versions) = &result {
                            debug!(
                                "Environment {:?} loaded: {} versions",
                                env_id,
                                versions.len(),
                            );
                        }
                        (env_id, result)
                    },
                    |(env_id, result)| Message::EnvironmentLoaded { env_id, result },
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
            env.loading = true;
            env.error = None;
            let env_id = env.id.clone();

            state.refresh_rotation = std::f32::consts::TAU / 40.0;
            let backend = state.backend.clone();
            let fetch_timeout = Duration::from_secs(self.settings.fetch_timeout_secs);

            return Task::perform(
                async move {
                    let result =
                        match tokio::time::timeout(fetch_timeout, backend.list_installed()).await {
                            Ok(Ok(versions)) => Ok(versions),
                            Ok(Err(error)) => {
                                Err(AppError::message(format!("Failed to load versions: {error}")))
                            }
                            Err(_) => {
                                Err(AppError::timeout("Loading versions", fetch_timeout.as_secs()))
                            }
                        };
                    (env_id, result)
                },
                |(env_id, result)| Message::EnvironmentLoaded { env_id, result },
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
                    _ => {}
                }
                state.active_filters.insert(filter);
            }
        }
    }
}
