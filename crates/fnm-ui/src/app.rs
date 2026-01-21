use std::path::PathBuf;
use std::time::Instant;

use iced::widget::{column, container, text};
use iced::{Element, Subscription, Task, Theme};

use fnm_core::{detect_fnm, fetch_release_schedule, FnmClient, InstallPhase, VersionGroup};
use fnm_platform::EnvironmentId;
use fnm_shell::detect_shells;

use crate::message::{InitResult, Message};
use crate::settings::{AppSettings, ThemeSetting};
use crate::state::{
    AppState, InstallModalState, MainState, Modal, OnboardingState, OnboardingStep, Operation,
    SettingsModalState, ShellConfigStatus, ShellSetupStatus, ShellVerificationStatus, Toast,
    ToastStatus, UndoAction,
};
use crate::theme::{dark_theme, get_system_theme, light_theme};
use crate::views;

pub struct FnmUi {
    state: AppState,
    settings: AppSettings,
}

impl FnmUi {
    pub fn new() -> (Self, Task<Message>) {
        let settings = AppSettings::load();

        let app = Self {
            state: AppState::Loading,
            settings,
        };

        let init_task = Task::perform(initialize(), Message::Initialized);

        (app, init_task)
    }

    pub fn title(&self) -> String {
        match &self.state {
            AppState::Loading => "fnm-ui".to_string(),
            AppState::Onboarding(_) => "fnm-ui - Setup".to_string(),
            AppState::Main(state) => {
                if let Some(v) = &state.active_environment().default_version {
                    format!("fnm-ui - Node {}", v)
                } else {
                    "fnm-ui".to_string()
                }
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Initialized(result) => self.handle_initialized(result),
            Message::EnvironmentLoaded {
                env_id,
                versions,
                default_version,
            } => self.handle_environment_loaded(env_id, versions, default_version),
            Message::EnvironmentError { env_id, error } => {
                self.handle_environment_error(env_id, error)
            }
            Message::RefreshEnvironment => self.handle_refresh_environment(),
            Message::VersionGroupToggled { major } => {
                self.handle_version_group_toggled(major);
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.handle_search_changed(query);
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
            Message::OpenInstallModal => {
                self.handle_open_install_modal();
                let fetch_versions = self.handle_fetch_remote_versions();
                let fetch_schedule = self.handle_fetch_release_schedule();
                Task::batch([fetch_versions, fetch_schedule])
            }
            Message::CloseModal => {
                self.handle_close_modal();
                Task::none()
            }
            Message::InstallModalSearchChanged(query) => {
                self.handle_install_modal_search_changed(query);
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
            Message::InstallProgress { version, progress } => {
                self.handle_install_progress(version, progress);
                Task::none()
            }
            Message::InstallComplete {
                version,
                success,
                error,
            } => self.handle_install_complete(version, success, error),
            Message::RequestUninstall(version) => {
                self.handle_request_uninstall(version);
                Task::none()
            }
            Message::ConfirmUninstall(version) => self.handle_confirm_uninstall(version),
            Message::CancelUninstall => {
                self.handle_close_modal();
                Task::none()
            }
            Message::UninstallComplete {
                version,
                success,
                error,
            } => self.handle_uninstall_complete(version, success, error),
            Message::SetDefault(version) => self.handle_set_default(version),
            Message::DefaultChanged {
                version,
                previous,
                success,
                error,
            } => self.handle_default_changed(version, previous, success, error),
            Message::ToastTimeout(id) | Message::ToastDismiss(id) => {
                if let AppState::Main(state) = &mut self.state {
                    state.remove_toast(id);
                }
                Task::none()
            }
            Message::ToastUndo(id) => self.handle_toast_undo(id),
            Message::OpenSettings => {
                if let AppState::Main(state) = &mut self.state {
                    let mut settings_state = SettingsModalState::new();
                    settings_state.checking_shells = true;
                    state.modal = Some(Modal::Settings(settings_state));
                }
                self.handle_check_shell_setup()
            }
            Message::CloseSettings => {
                self.handle_close_modal();
                Task::none()
            }
            Message::ThemeChanged(theme) => {
                self.settings.theme = theme;
                let _ = self.settings.save();
                Task::none()
            }
            Message::CheckShellSetup => self.handle_check_shell_setup(),
            Message::ShellSetupChecked(results) => {
                self.handle_shell_setup_checked(results);
                Task::none()
            }
            Message::ConfigureShell(shell_type) => self.handle_configure_shell(shell_type),
            Message::ShellConfigured(shell_type, result) => {
                self.handle_shell_configured(shell_type, result);
                Task::none()
            }
            Message::OnboardingNext => self.handle_onboarding_next(),
            Message::OnboardingBack => {
                self.handle_onboarding_back();
                Task::none()
            }
            Message::OnboardingInstallFnm => self.handle_onboarding_install_fnm(),
            Message::OnboardingFnmInstallResult(result) => {
                self.handle_onboarding_fnm_install_result(result)
            }
            Message::OnboardingConfigureShell(shell_type) => {
                self.handle_onboarding_configure_shell(shell_type)
            }
            Message::OnboardingShellConfigResult(result) => {
                self.handle_onboarding_shell_config_result(result);
                Task::none()
            }
            Message::OnboardingComplete => self.handle_onboarding_complete(),
            Message::Tick => {
                if let AppState::Main(state) = &mut self.state {
                    state.toasts.retain(|t| !t.is_expired());
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        match &self.state {
            AppState::Loading => views::loading::view(),
            AppState::Onboarding(state) => views::onboarding::view(state),
            AppState::Main(state) => views::main_view::view(state, &self.settings),
        }
    }

    pub fn theme(&self) -> Theme {
        match self.settings.theme {
            ThemeSetting::System => get_system_theme(),
            ThemeSetting::Light => light_theme(),
            ThemeSetting::Dark => dark_theme(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);

        tick
    }

    fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        if !result.fnm_found {
            let shells = detect_shells();
            let shell_statuses: Vec<ShellConfigStatus> = shells
                .into_iter()
                .map(|s| ShellConfigStatus {
                    shell_type: s.shell_type.clone(),
                    shell_name: s.shell_type.name().to_string(),
                    configured: s.is_configured,
                    config_path: s.config_file,
                    configuring: false,
                    error: None,
                })
                .collect();

            let mut onboarding = OnboardingState::new();
            onboarding.detected_shells = shell_statuses;
            self.state = AppState::Onboarding(onboarding);
            return Task::none();
        }

        let fnm_path = PathBuf::from("fnm");
        self.state = AppState::Main(MainState::new(fnm_path.clone()));

        let client = FnmClient::new(fnm_path.clone());
        let load_installed = Task::perform(
            async move {
                let versions = client.list_installed().await.unwrap_or_default();
                let default = client.default_version().await.ok().flatten();
                (versions, default)
            },
            move |(versions, default)| Message::EnvironmentLoaded {
                env_id: EnvironmentId::Native,
                versions,
                default_version: default,
            },
        );

        let fetch_remote = self.handle_fetch_remote_versions();

        Task::batch([load_installed, fetch_remote])
    }

    fn handle_environment_loaded(
        &mut self,
        env_id: EnvironmentId,
        versions: Vec<fnm_core::InstalledVersion>,
        _default_version: Option<fnm_core::NodeVersion>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                env.update_versions(versions);
            }
        }
        Task::none()
    }

    fn handle_environment_error(&mut self, env_id: EnvironmentId, error: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if let Some(env) = state.environments.iter_mut().find(|e| e.id == env_id) {
                env.loading = false;
                env.error = Some(error);
            }
        }
        Task::none()
    }

    fn handle_refresh_environment(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            env.loading = true;
            env.error = None;

            let env_id = env.id.clone();
            let fnm_path = state.fnm_path.clone();
            let client = FnmClient::new(fnm_path);

            return Task::perform(
                async move {
                    let versions = client.list_installed().await.unwrap_or_default();
                    let default = client.default_version().await.ok().flatten();
                    (env_id, versions, default)
                },
                |(env_id, versions, default)| Message::EnvironmentLoaded {
                    env_id,
                    versions,
                    default_version: default,
                },
            );
        }
        Task::none()
    }

    fn handle_version_group_toggled(&mut self, major: u32) {
        if let AppState::Main(state) = &mut self.state {
            let env = state.active_environment_mut();
            if let Some(group) = env.version_groups.iter_mut().find(|g| g.major == major) {
                group.is_expanded = !group.is_expanded;
            }
        }
    }

    fn handle_search_changed(&mut self, query: String) {
        if let AppState::Main(state) = &mut self.state {
            state.search_query = query;
        }
    }

    fn handle_fetch_remote_versions(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.available_versions.loading {
                return Task::none();
            }
            state.available_versions.loading = true;

            let fnm_path = state.fnm_path.clone();
            let client = FnmClient::new(fnm_path);

            return Task::perform(
                async move { client.list_remote().await.map_err(|e| e.to_string()) },
                Message::RemoteVersionsFetched,
            );
        }
        Task::none()
    }

    fn handle_remote_versions_fetched(
        &mut self,
        result: Result<Vec<fnm_core::RemoteVersion>, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            state.available_versions.loading = false;
            match result {
                Ok(versions) => {
                    state.available_versions.versions = versions.clone();
                    state.available_versions.fetched_at = Some(Instant::now());
                    state.available_versions.error = None;

                    if let Some(Modal::Install(modal_state)) = &mut state.modal {
                        modal_state.filtered_versions = versions;
                        modal_state.loading = false;
                    }
                }
                Err(error) => {
                    state.available_versions.error = Some(error);
                }
            }
        }
    }

    fn handle_fetch_release_schedule(&mut self) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if state.available_versions.schedule.is_some() {
                return Task::none();
            }

            return Task::perform(
                async move { fetch_release_schedule().await },
                Message::ReleaseScheduleFetched,
            );
        }
        Task::none()
    }

    fn handle_release_schedule_fetched(
        &mut self,
        result: Result<fnm_core::ReleaseSchedule, String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if let Ok(schedule) = result {
                state.available_versions.schedule = Some(schedule.clone());

                if let Some(Modal::Install(modal_state)) = &mut state.modal {
                    modal_state.schedule = Some(schedule);
                }
            }
        }
    }

    fn handle_open_install_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            let mut modal_state = InstallModalState::new();
            modal_state.loading = true;
            modal_state.filtered_versions = state.available_versions.versions.clone();
            modal_state.schedule = state.available_versions.schedule.clone();
            state.modal = Some(Modal::Install(modal_state));
        }
    }

    fn handle_close_modal(&mut self) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
        }
    }

    fn handle_install_modal_search_changed(&mut self, query: String) {
        if let AppState::Main(state) = &mut self.state {
            if let Some(Modal::Install(modal_state)) = &mut state.modal {
                modal_state.search_query = query.clone();

                let query_lower = query.to_lowercase();
                let schedule = &state.available_versions.schedule;

                modal_state.filtered_versions = state
                    .available_versions
                    .versions
                    .iter()
                    .filter(|v| {
                        let major = v.version.major;
                        let is_active = schedule
                            .as_ref()
                            .map(|s| s.is_active(major))
                            .unwrap_or(true);

                        if query_lower == "lts" {
                            let is_lts = schedule
                                .as_ref()
                                .map(|s| s.is_lts(major))
                                .unwrap_or(v.lts_codename.is_some());
                            return is_lts && is_active;
                        }
                        if query_lower == "latest" {
                            return v.is_latest;
                        }

                        let version_str = v.version.to_string();
                        let matches_query = version_str.contains(&query)
                            || v.lts_codename
                                .as_ref()
                                .map(|c| c.to_lowercase().contains(&query_lower))
                                .unwrap_or(false);

                        matches_query && is_active
                    })
                    .cloned()
                    .collect();
            }
        }
    }

    fn handle_start_install(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
            state.current_operation = Some(Operation::Install {
                version: version.clone(),
                progress: Default::default(),
            });

            let fnm_path = state.fnm_path.clone();
            let client = FnmClient::new(fnm_path);
            let version_clone = version.clone();

            return Task::run(
                async_stream::stream! {
                    match client.install_with_progress(&version_clone).await {
                        Ok(mut rx) => {
                            let mut final_success = false;
                            while let Some(progress) = rx.recv().await {
                                let is_complete = progress.phase == fnm_core::InstallPhase::Complete;
                                let is_failed = progress.phase == fnm_core::InstallPhase::Failed;

                                yield Message::InstallProgress {
                                    version: version_clone.clone(),
                                    progress,
                                };

                                if is_complete {
                                    final_success = true;
                                    break;
                                }
                                if is_failed {
                                    break;
                                }
                            }
                            yield Message::InstallComplete {
                                version: version_clone.clone(),
                                success: final_success,
                                error: if final_success { None } else { Some("Installation failed".to_string()) },
                            };
                        }
                        Err(e) => {
                            yield Message::InstallComplete {
                                version: version_clone.clone(),
                                success: false,
                                error: Some(e.to_string()),
                            };
                        }
                    }
                },
                |msg| msg,
            );
        }
        Task::none()
    }

    fn handle_install_progress(&mut self, _version: String, progress: fnm_core::InstallProgress) {
        if let AppState::Main(state) = &mut self.state {
            if let Some(Operation::Install {
                progress: op_progress,
                ..
            }) = &mut state.current_operation
            {
                *op_progress = progress;
            }
        }
    }

    fn handle_install_complete(
        &mut self,
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.current_operation = None;

            let toast_id = state.next_toast_id();
            if success {
                state.add_toast(Toast::success(
                    toast_id,
                    format!("Node {} installed successfully", version),
                ));
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to install Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }

            if success {
                return self.handle_refresh_environment();
            }
        }
        Task::none()
    }

    fn handle_request_uninstall(&mut self, version: String) {
        if let AppState::Main(state) = &mut self.state {
            state.modal = Some(Modal::ConfirmUninstall { version });
        }
    }

    fn handle_confirm_uninstall(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.modal = None;
            state.current_operation = Some(Operation::Uninstall {
                version: version.clone(),
            });

            let fnm_path = state.fnm_path.clone();
            let client = FnmClient::new(fnm_path);
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match client.uninstall(&version_clone).await {
                        Ok(()) => (version_clone, true, None),
                        Err(e) => (version_clone, false, Some(e.to_string())),
                    }
                },
                |(version, success, error)| Message::UninstallComplete {
                    version,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    fn handle_uninstall_complete(
        &mut self,
        version: String,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.current_operation = None;

            let toast_id = state.next_toast_id();
            if success {
                let toast = Toast::success(toast_id, format!("Node {} uninstalled", version))
                    .with_undo(UndoAction::Reinstall {
                        version: version.clone(),
                    });
                state.add_toast(toast);
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!(
                        "Failed to uninstall Node {}: {}",
                        version,
                        error.unwrap_or_default()
                    ),
                ));
            }

            if success {
                return self.handle_refresh_environment();
            }
        }
        Task::none()
    }

    fn handle_set_default(&mut self, version: String) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let previous = state
                .active_environment()
                .default_version
                .as_ref()
                .map(|v| v.to_string());

            state.current_operation = Some(Operation::SetDefault {
                version: version.clone(),
                previous: previous.clone(),
            });

            let fnm_path = state.fnm_path.clone();
            let client = FnmClient::new(fnm_path);
            let version_clone = version.clone();

            return Task::perform(
                async move {
                    match client.set_default(&version_clone).await {
                        Ok(()) => (version_clone, previous, true, None),
                        Err(e) => (version_clone, previous, false, Some(e.to_string())),
                    }
                },
                |(version, previous, success, error)| Message::DefaultChanged {
                    version,
                    previous,
                    success,
                    error,
                },
            );
        }
        Task::none()
    }

    fn handle_default_changed(
        &mut self,
        version: String,
        previous: Option<String>,
        success: bool,
        error: Option<String>,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            state.current_operation = None;

            let toast_id = state.next_toast_id();
            if success {
                let mut toast =
                    Toast::success(toast_id, format!("Default set to Node {}", version));
                if let Some(prev) = previous {
                    toast = toast.with_undo(UndoAction::ResetDefault { version: prev });
                }
                state.add_toast(toast);
                return self.handle_refresh_environment();
            } else {
                state.add_toast(Toast::error(
                    toast_id,
                    format!("Failed to set default: {}", error.unwrap_or_default()),
                ));
            }
        }
        Task::none()
    }

    fn handle_toast_undo(&mut self, id: usize) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            let toast = state.toasts.iter().find(|t| t.id == id).cloned();
            state.remove_toast(id);

            if let Some(toast) = toast {
                if let Some(undo_action) = toast.undo_action {
                    match undo_action {
                        UndoAction::Reinstall { version } => {
                            return self.handle_start_install(version);
                        }
                        UndoAction::ResetDefault { version } => {
                            return self.handle_set_default(version);
                        }
                    }
                }
            }
        }
        Task::none()
    }

    fn handle_onboarding_next(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::InstallFnm,
                OnboardingStep::InstallFnm => OnboardingStep::ConfigureShell,
                OnboardingStep::ConfigureShell => OnboardingStep::InstallNode,
                OnboardingStep::InstallNode => OnboardingStep::Complete,
                OnboardingStep::Complete => return self.handle_onboarding_complete(),
            };
        }
        Task::none()
    }

    fn handle_onboarding_back(&mut self) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::Welcome,
                OnboardingStep::InstallFnm => OnboardingStep::Welcome,
                OnboardingStep::ConfigureShell => OnboardingStep::InstallFnm,
                OnboardingStep::InstallNode => OnboardingStep::ConfigureShell,
                OnboardingStep::Complete => OnboardingStep::InstallNode,
            };
        }
    }

    fn handle_onboarding_install_fnm(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.fnm_installing = true;
            state.install_error = None;

            return Task::perform(
                async move { fnm_core::install_fnm().await.map_err(|e| e.to_string()) },
                Message::OnboardingFnmInstallResult,
            );
        }
        Task::none()
    }

    fn handle_onboarding_fnm_install_result(
        &mut self,
        result: Result<(), String>,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.fnm_installing = false;
            match result {
                Ok(()) => {
                    state.step = OnboardingStep::ConfigureShell;
                }
                Err(error) => {
                    state.install_error = Some(error);
                }
            }
        }
        Task::none()
    }

    fn handle_onboarding_configure_shell(
        &mut self,
        shell_type: fnm_shell::ShellType,
    ) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            if let Some(shell) = state
                .detected_shells
                .iter_mut()
                .find(|s| s.shell_type == shell_type)
            {
                shell.configuring = true;
                shell.error = None;
            }

            return Task::perform(
                async move {
                    use fnm_shell::{get_or_create_config_path, ShellConfig};

                    let config_path = get_or_create_config_path(&shell_type)
                        .ok_or_else(|| "No config file path found".to_string())?;

                    let mut config =
                        ShellConfig::load(shell_type, config_path).map_err(|e| e.to_string())?;

                    let edit = config.add_fnm_init();
                    if edit.has_changes() {
                        config.apply_edit(&edit).map_err(|e| e.to_string())?;
                    }

                    Ok(())
                },
                Message::OnboardingShellConfigResult,
            );
        }
        Task::none()
    }

    fn handle_onboarding_shell_config_result(&mut self, result: Result<(), String>) {
        if let AppState::Onboarding(state) = &mut self.state {
            for shell in &mut state.detected_shells {
                if shell.configuring {
                    shell.configuring = false;
                    match &result {
                        Ok(()) => {
                            shell.configured = true;
                            shell.error = None;
                        }
                        Err(error) => {
                            shell.error = Some(error.clone());
                        }
                    }
                    break;
                }
            }
        }
    }

    fn handle_onboarding_complete(&mut self) -> Task<Message> {
        let fnm_path = PathBuf::from("fnm");
        self.state = AppState::Main(MainState::new(fnm_path.clone()));

        let client = FnmClient::new(fnm_path);
        Task::perform(
            async move {
                let versions = client.list_installed().await.unwrap_or_default();
                let default = client.default_version().await.ok().flatten();
                (versions, default)
            },
            move |(versions, default)| Message::EnvironmentLoaded {
                env_id: EnvironmentId::Native,
                versions,
                default_version: default,
            },
        )
    }

    fn handle_check_shell_setup(&mut self) -> Task<Message> {
        use fnm_shell::{detect_shells, verify_shell_config};

        Task::perform(
            async move {
                let shells = detect_shells();
                let mut results = Vec::new();

                for shell in shells {
                    let result = verify_shell_config(&shell.shell_type).await;
                    results.push((shell.shell_type, result));
                }

                results
            },
            Message::ShellSetupChecked,
        )
    }

    fn handle_shell_setup_checked(
        &mut self,
        results: Vec<(fnm_shell::ShellType, fnm_shell::VerificationResult)>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if let Some(Modal::Settings(settings_state)) = &mut state.modal {
                settings_state.checking_shells = false;
                settings_state.shell_statuses = results
                    .into_iter()
                    .map(|(shell_type, result)| {
                        let status = match result {
                            fnm_shell::VerificationResult::Configured => {
                                ShellVerificationStatus::Configured
                            }
                            fnm_shell::VerificationResult::NotConfigured => {
                                ShellVerificationStatus::NotConfigured
                            }
                            fnm_shell::VerificationResult::ConfigFileNotFound => {
                                ShellVerificationStatus::NotConfigured
                            }
                            fnm_shell::VerificationResult::FunctionalButNotInConfig => {
                                ShellVerificationStatus::FunctionalButNotInConfig
                            }
                            fnm_shell::VerificationResult::Error(e) => {
                                ShellVerificationStatus::Error(e)
                            }
                        };
                        ShellSetupStatus {
                            shell_name: shell_type.name().to_string(),
                            shell_type,
                            status,
                            configuring: false,
                        }
                    })
                    .collect();
            }
        }
    }

    fn handle_configure_shell(&mut self, shell_type: fnm_shell::ShellType) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state {
            if let Some(Modal::Settings(settings_state)) = &mut state.modal {
                if let Some(shell) = settings_state
                    .shell_statuses
                    .iter_mut()
                    .find(|s| s.shell_type == shell_type)
                {
                    shell.configuring = true;
                }
            }
        }

        let shell_type_for_callback = shell_type.clone();
        Task::perform(
            async move {
                use fnm_shell::{get_or_create_config_path, ShellConfig};

                let config_path = get_or_create_config_path(&shell_type)
                    .ok_or_else(|| "No config file path found".to_string())?;

                let mut config = ShellConfig::load(shell_type.clone(), config_path)
                    .map_err(|e| e.to_string())?;

                let edit = config.add_fnm_init();
                if edit.has_changes() {
                    config.apply_edit(&edit).map_err(|e| e.to_string())?;
                }

                Ok::<_, String>(())
            },
            move |result| Message::ShellConfigured(shell_type_for_callback.clone(), result),
        )
    }

    fn handle_shell_configured(
        &mut self,
        shell_type: fnm_shell::ShellType,
        result: Result<(), String>,
    ) {
        if let AppState::Main(state) = &mut self.state {
            if let Some(Modal::Settings(settings_state)) = &mut state.modal {
                if let Some(shell) = settings_state
                    .shell_statuses
                    .iter_mut()
                    .find(|s| s.shell_type == shell_type)
                {
                    shell.configuring = false;
                    match result {
                        Ok(()) => shell.status = ShellVerificationStatus::Configured,
                        Err(e) => shell.status = ShellVerificationStatus::Error(e),
                    }
                }
            }
        }
    }
}

async fn initialize() -> InitResult {
    let detection = detect_fnm().await;

    InitResult {
        fnm_found: detection.found,
        fnm_version: detection.version,
        environments: vec![EnvironmentId::Native],
    }
}
