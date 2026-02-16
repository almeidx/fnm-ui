use iced::Task;
use log::info;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::super::{Versi, platform};

impl Versi {
    #[allow(clippy::too_many_lines)]
    pub(super) fn dispatch_settings(&mut self, message: Message) -> super::DispatchResult {
        match message {
            Message::ToastDismiss(id) => {
                if let AppState::Main(state) = &mut self.state {
                    state.remove_toast(id);
                }
                Ok(Task::none())
            }
            Message::NavigateToVersions => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                Ok(Task::none())
            }
            Message::NavigateToSettings => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Settings;
                    state.settings_state.checking_shells = true;
                }
                let shell_task = self.handle_check_shell_setup();
                let log_stats_task = Task::perform(
                    async {
                        let log_path = versi_platform::AppPaths::new().ok()?.log_file();
                        std::fs::metadata(&log_path).ok().map(|m| m.len())
                    },
                    Message::LogFileStatsLoaded,
                );
                Ok(Task::batch([shell_task, log_stats_task]))
            }
            Message::NavigateToAbout => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::About;
                }
                Ok(Task::none())
            }
            Message::VersionRowHovered(version) => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.hovered_version = None;
                    } else {
                        state.hovered_version = version;
                    }
                }
                Ok(Task::none())
            }
            Message::ThemeChanged(theme) => {
                self.settings.theme = theme;
                self.save_settings_with_log();
                Ok(Task::none())
            }
            Message::ShellOptionUseOnCdToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.use_on_cd = value))
            }
            Message::ShellOptionResolveEnginesToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.resolve_engines = value))
            }
            Message::ShellOptionCorepackEnabledToggled(value) => {
                Ok(self.update_active_shell_options(|options| options.corepack_enabled = value))
            }
            Message::DebugLoggingToggled(value) => {
                self.settings.debug_logging = value;
                self.save_settings_with_log();
                crate::logging::set_logging_enabled(value);
                if value {
                    info!("Debug logging enabled");
                }
                Ok(Task::none())
            }
            Message::CopyToClipboard(text) => Ok(iced::clipboard::write(text)),
            Message::ClearLogFile => {
                let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file())
                else {
                    return Ok(Task::none());
                };
                Ok(Task::perform(
                    async move {
                        if log_path.exists() {
                            let _ = std::fs::write(&log_path, "");
                        }
                    },
                    |()| Message::LogFileCleared,
                ))
            }
            Message::LogFileCleared => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = Some(0);
                }
                Ok(Task::none())
            }
            Message::RevealLogFile => {
                let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file())
                else {
                    return Ok(Task::none());
                };
                Ok(Task::perform(
                    async move { platform::reveal_in_file_manager(&log_path) },
                    |()| Message::NoOp,
                ))
            }
            Message::RevealSettingsFile => {
                self.save_settings_with_log();
                let Some(settings_path) = versi_platform::AppPaths::new()
                    .ok()
                    .map(|p| p.settings_file())
                else {
                    return Ok(Task::none());
                };
                Ok(Task::perform(
                    async move { platform::reveal_in_file_manager(&settings_path) },
                    |()| Message::NoOp,
                ))
            }
            Message::LogFileStatsLoaded(size) => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = size;
                }
                Ok(Task::none())
            }
            Message::ShellFlagsUpdated => Ok(Task::none()),
            Message::ExportSettings => Ok(self.handle_export_settings()),
            Message::SettingsExported(result) => Ok(self.handle_settings_exported(result)),
            Message::ImportSettings => Ok(Self::handle_import_settings()),
            Message::SettingsImported(result) => Ok(self.handle_settings_imported(result)),
            Message::ShellSetupChecked(results) => {
                self.handle_shell_setup_checked(results);
                Ok(Task::none())
            }
            Message::ConfigureShell(shell_type) => Ok(self.handle_configure_shell(shell_type)),
            Message::ShellConfigured(shell_type, result) => {
                self.handle_shell_configured(&shell_type, &result);
                Ok(Task::none())
            }
            Message::PreferredBackendChanged(name) => {
                Ok(self.handle_preferred_backend_changed(name))
            }
            Message::OnboardingNext => Ok(self.handle_onboarding_next()),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Ok(Task::none())
            }
            Message::OnboardingSelectBackend(name) => {
                self.handle_onboarding_select_backend(name);
                Ok(Task::none())
            }
            Message::OnboardingInstallBackend => Ok(self.handle_onboarding_install_backend()),
            Message::OnboardingBackendInstallResult(result) => {
                Ok(self.handle_onboarding_backend_install_result(result))
            }
            Message::OnboardingConfigureShell(shell_type) => {
                Ok(self.handle_onboarding_configure_shell(shell_type))
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(&result);
                Ok(Task::none())
            }
            Message::OnboardingComplete => Ok(self.handle_onboarding_complete()),
            Message::TrayBehaviorChanged(behavior) => {
                Ok(self.handle_tray_behavior_changed(behavior))
            }
            Message::StartMinimizedToggled(value) => {
                self.settings.start_minimized = value;
                self.save_settings_with_log();
                Ok(Task::none())
            }
            Message::LaunchAtLoginToggled(value) => {
                self.settings.launch_at_login = value;
                if let Err(e) = platform::set_launch_at_login(value) {
                    log::error!("Failed to set launch at login: {e}");
                }
                self.save_settings_with_log();
                Ok(Task::none())
            }
            Message::SystemThemeChanged(mode) => {
                self.system_theme_mode = mode;
                Ok(Task::none())
            }
            other => Err(Box::new(other)),
        }
    }

    fn update_active_shell_options<F>(&mut self, update: F) -> Task<Message>
    where
        F: FnOnce(&mut crate::settings::ShellOptions),
    {
        let backend_kind = self.active_backend_kind();
        update(self.settings.shell_options_for_mut(backend_kind));
        self.save_settings_with_log();
        self.update_shell_flags()
    }

    fn save_settings_with_log(&self) {
        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {e}");
        }
    }
}
