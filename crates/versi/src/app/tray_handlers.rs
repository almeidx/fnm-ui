//! System tray event handling and menu updates.
//!
//! Handles messages: `TrayEvent`, `TrayBehaviorChanged`

use log::error;

use iced::Task;

use crate::message::Message;
use crate::settings::TrayBehavior;
use crate::state::{AppState, MainViewKind};
use crate::tray::{self, TrayMenuData, TrayMessage};

use super::Versi;
use super::init::create_backend_for_environment;
use super::platform;

impl Versi {
    pub(super) fn handle_tray_event(&mut self, msg: TrayMessage) -> Task<Message> {
        match msg {
            TrayMessage::ShowWindow => self.tray_show_window(),
            TrayMessage::HideWindow => self.tray_hide_window(),
            TrayMessage::Quit => iced::exit(),
            _ if !matches!(self.state, AppState::Main(_)) => Task::none(),
            TrayMessage::OpenSettings => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Settings;
                    state.settings_state.checking_shells = true;
                }
                let show_task = if let Some(id) = self.window_id {
                    platform::set_dock_visible(true);
                    Task::batch([
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::minimize(id, false),
                        iced::window::gain_focus(id),
                    ])
                } else {
                    Task::none()
                };
                let shell_task = self.handle_check_shell_setup();
                let log_stats_task = Task::perform(
                    async {
                        let log_path = versi_platform::AppPaths::new().ok()?.log_file();
                        std::fs::metadata(&log_path).ok().map(|m| m.len())
                    },
                    Message::LogFileStatsLoaded,
                );
                Task::batch([show_task, shell_task, log_stats_task])
            }
            TrayMessage::OpenAbout => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::About;
                }
                if let Some(id) = self.window_id {
                    platform::set_dock_visible(true);
                    Task::batch([
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::minimize(id, false),
                        iced::window::gain_focus(id),
                    ])
                } else {
                    Task::none()
                }
            }
            TrayMessage::SetDefault { env_index, version } => {
                let mut switched_env: Option<(
                    versi_platform::EnvironmentId,
                    crate::backend_kind::BackendKind,
                )> = None;

                if let AppState::Main(state) = &mut self.state
                    && env_index != state.active_environment_idx
                    && let Some(env) = state.environments.get(env_index)
                {
                    state.active_environment_idx = env_index;
                    state.backend_name = env.backend_name;
                    state.backend_update = None;
                    switched_env = Some((env.id.clone(), env.backend_name));
                }

                if let Some((env_id, backend_name)) = switched_env {
                    let env_provider = self.provider_for_kind(backend_name);
                    self.provider = env_provider.clone();
                    if let AppState::Main(state) = &mut self.state {
                        state.backend = create_backend_for_environment(
                            &env_id,
                            &self.backend_path,
                            self.backend_dir.as_ref(),
                            &env_provider,
                        );
                    }
                }
                self.handle_set_default(version)
            }
        }
    }

    fn tray_show_window(&mut self) -> Task<Message> {
        self.pending_minimize = false;
        self.window_visible = true;
        self.update_tray_menu();

        let needs_refresh = if let AppState::Main(state) = &self.state {
            state.active_environment().installed_versions.is_empty()
                && !state.active_environment().loading
        } else {
            false
        };

        if let Some(id) = self.window_id {
            platform::set_dock_visible(true);
            let mut tasks = vec![
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ];
            if needs_refresh {
                tasks.push(Task::done(Message::RefreshEnvironment));
            }
            Task::batch(tasks)
        } else {
            self.pending_show = true;
            Task::none()
        }
    }

    fn tray_hide_window(&mut self) -> Task<Message> {
        self.window_visible = false;
        self.update_tray_menu();

        if let Some(id) = self.window_id {
            platform::set_dock_visible(false);
            if platform::is_wayland() {
                iced::window::minimize(id, true)
            } else {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            }
        } else {
            Task::none()
        }
    }

    pub(super) fn handle_tray_behavior_changed(&mut self, behavior: TrayBehavior) -> Task<Message> {
        let old_behavior = self.settings.tray_behavior;
        self.settings.tray_behavior = behavior;

        if behavior != TrayBehavior::AlwaysRunning && self.settings.launch_at_login {
            self.settings.launch_at_login = false;
            if let Err(e) = platform::set_launch_at_login(false) {
                log::error!("Failed to disable launch at login: {e}");
            }
        }

        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {e}");
        }

        if old_behavior == TrayBehavior::Disabled && behavior != TrayBehavior::Disabled {
            if let Err(e) = tray::init_tray(behavior) {
                error!("Failed to initialize tray: {e}");
            } else {
                self.update_tray_menu();
            }
        } else if behavior == TrayBehavior::Disabled {
            tray::destroy_tray();
        }

        Task::none()
    }

    pub(super) fn update_tray_menu(&self) {
        if let AppState::Main(state) = &self.state {
            let data = TrayMenuData::from_environments(&state.environments, self.window_visible);
            tray::update_menu(&data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_app_with_two_environments;
    use super::*;
    use crate::state::{AppState, MainViewKind, Operation};

    #[test]
    fn tray_open_settings_switches_view_and_marks_shell_check() {
        let mut app = test_app_with_two_environments();
        let _ = app.handle_tray_event(TrayMessage::OpenSettings);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.view, MainViewKind::Settings);
        assert!(state.settings_state.checking_shells);
    }

    #[test]
    fn tray_open_about_switches_view() {
        let mut app = test_app_with_two_environments();
        let _ = app.handle_tray_event(TrayMessage::OpenAbout);

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.view, MainViewKind::About);
    }

    #[test]
    fn tray_show_hide_window_without_window_id_updates_flags() {
        let mut app = test_app_with_two_environments();
        app.window_id = None;
        app.window_visible = false;
        app.pending_minimize = true;
        app.pending_show = false;

        let _ = app.handle_tray_event(TrayMessage::ShowWindow);
        assert!(app.window_visible);
        assert!(!app.pending_minimize);
        assert!(app.pending_show);

        let _ = app.handle_tray_event(TrayMessage::HideWindow);
        assert!(!app.window_visible);
    }

    #[test]
    fn tray_set_default_switches_environment_and_starts_default_operation() {
        let mut app = test_app_with_two_environments();
        let _ = app.handle_tray_event(TrayMessage::SetDefault {
            env_index: 1,
            version: "v20.11.0".to_string(),
        });

        let AppState::Main(state) = &app.state else {
            panic!("expected main state");
        };
        assert_eq!(state.active_environment_idx, 1);
        assert_eq!(state.backend_name, crate::backend_kind::BackendKind::Nvm);
        assert!(matches!(
            state.operation_queue.exclusive_op.as_ref(),
            Some(Operation::SetDefault { version }) if version == "v20.11.0"
        ));
        assert_eq!(app.provider.name(), "nvm");
    }
}
