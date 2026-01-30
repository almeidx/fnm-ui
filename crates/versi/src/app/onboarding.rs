use std::path::PathBuf;

use iced::Task;

use versi_core::{FnmBackend, VersionManager};
use versi_platform::EnvironmentId;

use crate::message::Message;
use crate::state::{AppState, MainState, OnboardingStep};

use super::Versi;

impl Versi {
    pub(super) fn handle_onboarding_next(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::InstallFnm,
                OnboardingStep::InstallFnm => OnboardingStep::ConfigureShell,
                OnboardingStep::ConfigureShell => return self.handle_onboarding_complete(),
            };
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_back(&mut self) {
        if let AppState::Onboarding(state) = &mut self.state {
            state.step = match state.step {
                OnboardingStep::Welcome => OnboardingStep::Welcome,
                OnboardingStep::InstallFnm => OnboardingStep::Welcome,
                OnboardingStep::ConfigureShell => OnboardingStep::InstallFnm,
            };
        }
    }

    pub(super) fn handle_onboarding_install_fnm(&mut self) -> Task<Message> {
        if let AppState::Onboarding(state) = &mut self.state {
            state.fnm_installing = true;
            state.install_error = None;

            return Task::perform(
                async move { versi_core::install_fnm().await.map_err(|e| e.to_string()) },
                Message::OnboardingFnmInstallResult,
            );
        }
        Task::none()
    }

    pub(super) fn handle_onboarding_fnm_install_result(
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

    pub(super) fn handle_onboarding_configure_shell(
        &mut self,
        shell_type: versi_shell::ShellType,
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

            let shell_options = versi_shell::FnmShellOptions {
                use_on_cd: self.settings.shell_options.use_on_cd,
                resolve_engines: self.settings.shell_options.resolve_engines,
                corepack_enabled: self.settings.shell_options.corepack_enabled,
            };

            return Task::perform(
                async move {
                    use versi_shell::{ShellConfig, get_or_create_config_path};

                    let config_path = get_or_create_config_path(&shell_type)
                        .ok_or_else(|| "No config file path found".to_string())?;

                    let mut config =
                        ShellConfig::load(shell_type, config_path).map_err(|e| e.to_string())?;

                    let edit = config.add_fnm_init(&shell_options);
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

    pub(super) fn handle_onboarding_shell_config_result(&mut self, result: Result<(), String>) {
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

    pub(super) fn handle_onboarding_complete(&mut self) -> Task<Message> {
        let fnm_path = PathBuf::from("fnm");
        let fnm_dir = versi_core::detect_fnm_dir();

        let backend = FnmBackend::new(fnm_path.clone(), None, fnm_dir.clone());
        let backend = if let Some(dir) = fnm_dir.clone() {
            backend.with_fnm_dir(dir)
        } else {
            backend
        };
        let backend: Box<dyn VersionManager> = Box::new(backend.clone());
        let mut main_state = MainState::new(backend, None);
        main_state.search_query = "lts".to_string();
        self.state = AppState::Main(main_state);

        let load_backend = FnmBackend::new(fnm_path, None, fnm_dir.clone());
        let load_backend = if let Some(dir) = fnm_dir {
            load_backend.with_fnm_dir(dir)
        } else {
            load_backend
        };
        let load_task = Task::perform(
            async move {
                let versions = load_backend.list_installed().await.unwrap_or_default();
                (EnvironmentId::Native, versions)
            },
            |(env_id, versions)| Message::EnvironmentLoaded { env_id, versions },
        );

        let fetch_remote = self.handle_fetch_remote_versions();
        let fetch_schedule = self.handle_fetch_release_schedule();

        Task::batch([load_task, fetch_remote, fetch_schedule])
    }
}
