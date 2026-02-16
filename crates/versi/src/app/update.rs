use iced::Task;
use log::info;

use crate::message::Message;
use crate::state::{AppState, MainViewKind};

use super::{Versi, platform, should_dismiss_context_menu};

impl Versi {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && state.context_menu.is_some()
        {
            if should_dismiss_context_menu(&message) {
                state.context_menu = None;
            }
        }

        match message {
            Message::Initialized(result) => self.handle_initialized(result),
            Message::EnvironmentLoaded { env_id, result } => {
                self.handle_environment_loaded(env_id, result)
            }
            Message::RefreshEnvironment => self.handle_refresh_environment(),
            Message::FocusSearch => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                iced::widget::operation::focus(iced::widget::Id::new(
                    crate::views::main_view::search::SEARCH_INPUT_ID,
                ))
            }
            Message::SelectPreviousVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| i.saturating_sub(1))
                                .unwrap_or(0),
                            None => versions.len() - 1,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Task::none()
            }
            Message::SelectNextVersion => {
                if let AppState::Main(state) = &mut self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                {
                    let versions = state.navigable_versions(self.settings.search_results_limit);
                    if !versions.is_empty() {
                        let new_idx = match &state.hovered_version {
                            Some(current) => versions
                                .iter()
                                .position(|v| v == current)
                                .map(|i| (i + 1).min(versions.len() - 1))
                                .unwrap_or(0),
                            None => 0,
                        };
                        state.hovered_version = Some(versions[new_idx].clone());
                    }
                }
                Task::none()
            }
            Message::ActivateSelectedVersion => {
                if let AppState::Main(state) = &self.state
                    && state.view == MainViewKind::Versions
                    && state.modal.is_none()
                    && let Some(version) = state.hovered_version.clone()
                {
                    if state.is_version_installed(&version) {
                        return self.update(Message::SetDefault(version));
                    } else {
                        return self.update(Message::StartInstall(version));
                    }
                }
                Task::none()
            }
            Message::VersionGroupToggled { major } => {
                self.handle_version_group_toggled(major);
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.handle_search_changed(query);
                Task::none()
            }
            Message::SearchFilterToggled(filter) => {
                self.handle_search_filter_toggled(filter);
                Task::none()
            }
            Message::FetchRemoteVersions => self.handle_fetch_remote_versions(),
            Message::RemoteVersionsFetched(result) => {
                self.handle_remote_versions_fetched(result);
                Task::none()
            }
            Message::ReleaseScheduleFetched(result) => {
                self.handle_release_schedule_fetched(result);
                Task::none()
            }
            Message::VersionMetadataFetched(result) => {
                self.handle_version_metadata_fetched(result);
                Task::none()
            }
            Message::ShowVersionDetail(version) => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::VersionDetail { version });
                }
                Task::none()
            }
            Message::CloseModal => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.modal = None;
                    } else if state.view == MainViewKind::About
                        || state.view == MainViewKind::Settings
                    {
                        state.view = MainViewKind::Versions;
                    }
                }
                Task::none()
            }
            Message::OpenChangelog(version) => {
                let url = format!("https://nodejs.org/en/blog/release/{}", version);
                Task::perform(
                    async move {
                        let _ = open::that(&url);
                    },
                    |_| Message::NoOp,
                )
            }
            Message::StartInstall(version) => self.handle_start_install(version),
            Message::InstallComplete {
                version,
                success,
                error,
            } => self.handle_install_complete(version, success, error),
            Message::RequestUninstall(version) => self.handle_uninstall(version),
            Message::ConfirmUninstallDefault(version) => {
                self.handle_confirm_uninstall_default(version)
            }
            Message::UninstallComplete {
                version,
                success,
                error,
            } => self.handle_uninstall_complete(version, success, error),
            Message::RequestBulkUpdateMajors => self.handle_request_bulk_update_majors(),
            Message::RequestBulkUninstallEOL => self.handle_request_bulk_uninstall_eol(),
            Message::RequestBulkUninstallMajor { major } => {
                self.handle_request_bulk_uninstall_major(major)
            }
            Message::ConfirmBulkUpdateMajors => self.handle_confirm_bulk_update_majors(),
            Message::ConfirmBulkUninstallEOL => self.handle_confirm_bulk_uninstall_eol(),
            Message::ConfirmBulkUninstallMajor { major } => {
                self.handle_confirm_bulk_uninstall_major(major)
            }
            Message::RequestBulkUninstallMajorExceptLatest { major } => {
                self.handle_request_bulk_uninstall_major_except_latest(major)
            }
            Message::ConfirmBulkUninstallMajorExceptLatest { major } => {
                self.handle_confirm_bulk_uninstall_major_except_latest(major)
            }
            Message::CancelBulkOperation => {
                self.handle_close_modal();
                Task::none()
            }
            Message::SetDefault(version) => self.handle_set_default(version),
            Message::DefaultChanged { success, error } => {
                self.handle_default_changed(success, error)
            }
            Message::ToastDismiss(id) => {
                if let AppState::Main(state) = &mut self.state {
                    state.remove_toast(id);
                }
                Task::none()
            }
            Message::NavigateToVersions => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::Versions;
                }
                Task::none()
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
                Task::batch([shell_task, log_stats_task])
            }
            Message::NavigateToAbout => {
                if let AppState::Main(state) = &mut self.state {
                    state.view = MainViewKind::About;
                }
                Task::none()
            }
            Message::VersionRowHovered(version) => {
                if let AppState::Main(state) = &mut self.state {
                    if state.modal.is_some() {
                        state.hovered_version = None;
                    } else {
                        state.hovered_version = version;
                    }
                }
                Task::none()
            }
            Message::ThemeChanged(theme) => {
                self.settings.theme = theme;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                Task::none()
            }
            Message::ShellOptionUseOnCdToggled(value) => {
                let backend_kind = self.active_backend_kind();
                self.settings.shell_options_for_mut(backend_kind).use_on_cd = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::ShellOptionResolveEnginesToggled(value) => {
                let backend_name = self.active_backend_kind();
                self.settings
                    .shell_options_for_mut(backend_name)
                    .resolve_engines = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::ShellOptionCorepackEnabledToggled(value) => {
                let backend_name = self.active_backend_kind();
                self.settings
                    .shell_options_for_mut(backend_name)
                    .corepack_enabled = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                self.update_shell_flags()
            }
            Message::DebugLoggingToggled(value) => {
                self.settings.debug_logging = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                crate::logging::set_logging_enabled(value);
                if value {
                    info!("Debug logging enabled");
                }
                Task::none()
            }
            Message::CopyToClipboard(text) => iced::clipboard::write(text),
            Message::ClearLogFile => {
                let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file())
                else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        if log_path.exists() {
                            let _ = std::fs::write(&log_path, "");
                        }
                    },
                    |_| Message::LogFileCleared,
                )
            }
            Message::LogFileCleared => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = Some(0);
                }
                Task::none()
            }
            Message::RevealLogFile => {
                let Some(log_path) = versi_platform::AppPaths::new().ok().map(|p| p.log_file())
                else {
                    return Task::none();
                };
                Task::perform(
                    async move { platform::reveal_in_file_manager(&log_path) },
                    |_| Message::NoOp,
                )
            }
            Message::RevealSettingsFile => {
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                let Some(settings_path) = versi_platform::AppPaths::new()
                    .ok()
                    .map(|p| p.settings_file())
                else {
                    return Task::none();
                };
                Task::perform(
                    async move { platform::reveal_in_file_manager(&settings_path) },
                    |_| Message::NoOp,
                )
            }
            Message::LogFileStatsLoaded(size) => {
                if let AppState::Main(state) = &mut self.state {
                    state.settings_state.log_file_size = size;
                }
                Task::none()
            }
            Message::ShellFlagsUpdated => Task::none(),
            Message::ExportSettings => self.handle_export_settings(),
            Message::SettingsExported(result) => self.handle_settings_exported(result),
            Message::ImportSettings => self.handle_import_settings(),
            Message::SettingsImported(result) => self.handle_settings_imported(result),
            Message::ShellSetupChecked(results) => {
                self.handle_shell_setup_checked(results);
                Task::none()
            }
            Message::ConfigureShell(shell_type) => self.handle_configure_shell(shell_type),
            Message::ShellConfigured(shell_type, result) => {
                self.handle_shell_configured(shell_type, result);
                Task::none()
            }
            Message::PreferredBackendChanged(name) => self.handle_preferred_backend_changed(name),
            Message::OnboardingNext => self.handle_onboarding_next(),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Task::none()
            }
            Message::OnboardingSelectBackend(name) => {
                self.handle_onboarding_select_backend(name);
                Task::none()
            }
            Message::OnboardingInstallBackend => self.handle_onboarding_install_backend(),
            Message::OnboardingBackendInstallResult(result) => {
                self.handle_onboarding_backend_install_result(result)
            }
            Message::OnboardingConfigureShell(shell_type) => {
                self.handle_onboarding_configure_shell(shell_type)
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(result);
                Task::none()
            }
            Message::OnboardingComplete => self.handle_onboarding_complete(),
            Message::AnimationTick => {
                if let AppState::Main(state) = &mut self.state {
                    let loading = state.active_environment().loading;
                    state.refresh_rotation += std::f32::consts::TAU / 40.0;
                    if !loading && state.refresh_rotation >= std::f32::consts::TAU {
                        state.refresh_rotation = 0.0;
                    }
                }
                Task::none()
            }
            Message::Tick => {
                #[cfg(target_os = "linux")]
                {
                    if crate::tray::is_tray_active() {
                        while gtk::events_pending() {
                            gtk::main_iteration();
                        }
                    }
                }
                if let AppState::Main(state) = &mut self.state {
                    let timeout = self.settings.toast_timeout_secs;
                    state.toasts.retain(|t| !t.is_expired(timeout));
                }
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::CloseRequested)
            | Message::WindowEvent(iced::window::Event::Closed)
            | Message::CloseWindow => self.handle_window_close(),
            Message::WindowEvent(iced::window::Event::Resized(size)) => {
                self.window_size = Some(size);
                Task::none()
            }
            Message::WindowEvent(iced::window::Event::Moved(point)) => {
                self.window_position = Some(point);
                Task::none()
            }
            Message::WindowOpened(id) => self.handle_window_opened(id),
            Message::HideDockIcon => {
                platform::set_dock_visible(false);
                Task::none()
            }
            Message::WindowEvent(_) => Task::none(),
            Message::AppUpdateChecked(result) => {
                self.handle_app_update_checked(result);
                Task::none()
            }
            Message::OpenAppUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.app_update
                {
                    let url = update.release_url.clone();
                    return Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |_| Message::NoOp,
                    );
                }
                Task::none()
            }
            Message::StartAppUpdate => self.handle_start_app_update(),
            Message::AppUpdateProgress { downloaded, total } => {
                self.handle_app_update_progress(downloaded, total);
                Task::none()
            }
            Message::AppUpdateExtracting => {
                self.handle_app_update_extracting();
                Task::none()
            }
            Message::AppUpdateApplying => {
                self.handle_app_update_applying();
                Task::none()
            }
            Message::AppUpdateComplete(result) => self.handle_app_update_complete(result),
            Message::RestartApp => self.handle_restart_app(),
            Message::BackendUpdateChecked(result) => {
                self.handle_backend_update_checked(result);
                Task::none()
            }
            Message::FetchReleaseSchedule => self.handle_fetch_release_schedule(),
            Message::OpenBackendUpdate => {
                if let AppState::Main(state) = &self.state
                    && let Some(update) = &state.backend_update
                {
                    let url = update.release_url.clone();
                    return Task::perform(
                        async move {
                            let _ = open::that(&url);
                        },
                        |_| Message::NoOp,
                    );
                }
                Task::none()
            }
            Message::VersionListCursorMoved(point) => {
                if let AppState::Main(state) = &mut self.state {
                    state.cursor_position = point;
                }
                Task::none()
            }
            Message::ShowContextMenu {
                version,
                is_installed,
                is_default,
            } => {
                if let AppState::Main(state) = &mut self.state {
                    state.context_menu = Some(crate::state::ContextMenu {
                        version,
                        is_installed,
                        is_default,
                        position: state.cursor_position,
                    });
                }
                Task::none()
            }
            Message::CloseContextMenu => {
                if let AppState::Main(state) = &mut self.state {
                    state.context_menu = None;
                }
                Task::none()
            }
            Message::ShowKeyboardShortcuts => {
                if let AppState::Main(state) = &mut self.state {
                    state.modal = Some(crate::state::Modal::KeyboardShortcuts);
                }
                Task::none()
            }
            Message::OpenLink(url) => Task::perform(
                async move {
                    let _ = open::that(&url);
                },
                |_| Message::NoOp,
            ),
            Message::EnvironmentSelected(idx) => self.handle_environment_selected(idx),
            Message::SelectNextEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let next = (state.active_environment_idx + 1) % state.environments.len();
                    return self.handle_environment_selected(next);
                }
                Task::none()
            }
            Message::SelectPreviousEnvironment => {
                if let AppState::Main(state) = &self.state
                    && state.environments.len() > 1
                {
                    let prev = if state.active_environment_idx == 0 {
                        state.environments.len() - 1
                    } else {
                        state.active_environment_idx - 1
                    };
                    return self.handle_environment_selected(prev);
                }
                Task::none()
            }
            Message::TrayEvent(tray_msg) => self.handle_tray_event(tray_msg),
            Message::TrayBehaviorChanged(behavior) => self.handle_tray_behavior_changed(behavior),
            Message::StartMinimizedToggled(value) => {
                self.settings.start_minimized = value;
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                Task::none()
            }
            Message::LaunchAtLoginToggled(value) => {
                self.settings.launch_at_login = value;
                if let Err(e) = platform::set_launch_at_login(value) {
                    log::error!("Failed to set launch at login: {e}");
                }
                if let Err(e) = self.settings.save() {
                    log::error!("Failed to save settings: {e}");
                }
                Task::none()
            }
            Message::SystemThemeChanged(mode) => {
                self.system_theme_mode = mode;
                Task::none()
            }
            _ => Task::none(),
        }
    }
}
