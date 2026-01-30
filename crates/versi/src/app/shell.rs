use iced::Task;

use versi_shell::detect_shells;

use crate::message::Message;
use crate::state::{AppState, ShellSetupStatus, ShellVerificationStatus};

use super::Versi;

impl Versi {
    pub(super) fn handle_check_shell_setup(&mut self) -> Task<Message> {
        use versi_shell::{detect_native_shells, verify_shell_config};
        #[cfg(target_os = "windows")]
        use versi_shell::{detect_wsl_shells, verify_wsl_shell_config};

        #[allow(unused_variables)]
        let env_id = if let AppState::Main(state) = &self.state {
            Some(state.active_environment().id.clone())
        } else {
            None
        };

        Task::perform(
            async move {
                #[cfg(target_os = "windows")]
                let (shells, wsl_distro) = match &env_id {
                    Some(EnvironmentId::Wsl { distro, .. }) => {
                        (detect_wsl_shells(distro), Some(distro.clone()))
                    }
                    _ => (detect_native_shells(), None),
                };
                #[cfg(not(target_os = "windows"))]
                let (shells, wsl_distro): (Vec<_>, Option<String>) = (detect_native_shells(), None);

                let mut results = Vec::new();

                for shell in shells {
                    #[cfg(target_os = "windows")]
                    let result = if let Some(ref distro) = wsl_distro {
                        verify_wsl_shell_config(&shell.shell_type, distro).await
                    } else {
                        verify_shell_config(&shell.shell_type).await
                    };
                    #[cfg(not(target_os = "windows"))]
                    let result = {
                        let _ = &wsl_distro;
                        verify_shell_config(&shell.shell_type).await
                    };
                    results.push((shell.shell_type, result));
                }

                results
            },
            Message::ShellSetupChecked,
        )
    }

    pub(super) fn handle_shell_setup_checked(
        &mut self,
        results: Vec<(versi_shell::ShellType, versi_shell::VerificationResult)>,
    ) {
        let mut first_detected_options: Option<versi_shell::FnmShellOptions> = None;

        if let AppState::Main(state) = &mut self.state {
            state.settings_state.checking_shells = false;
            state.settings_state.shell_statuses = results
                .into_iter()
                .map(|(shell_type, result)| {
                    let status = match result {
                        versi_shell::VerificationResult::Configured(options) => {
                            if first_detected_options.is_none() {
                                first_detected_options = options;
                            }
                            ShellVerificationStatus::Configured
                        }
                        versi_shell::VerificationResult::NotConfigured => {
                            ShellVerificationStatus::NotConfigured
                        }
                        versi_shell::VerificationResult::ConfigFileNotFound => {
                            ShellVerificationStatus::NoConfigFile
                        }
                        versi_shell::VerificationResult::FunctionalButNotInConfig => {
                            ShellVerificationStatus::FunctionalButNotInConfig
                        }
                        versi_shell::VerificationResult::Error(e) => {
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

        if let Some(options) = first_detected_options {
            self.settings.shell_options.use_on_cd = options.use_on_cd;
            self.settings.shell_options.resolve_engines = options.resolve_engines;
            self.settings.shell_options.corepack_enabled = options.corepack_enabled;
        }
    }

    pub(super) fn handle_configure_shell(
        &mut self,
        shell_type: versi_shell::ShellType,
    ) -> Task<Message> {
        if let AppState::Main(state) = &mut self.state
            && let Some(shell) = state
                .settings_state
                .shell_statuses
                .iter_mut()
                .find(|s| s.shell_type == shell_type)
        {
            shell.configuring = true;
        }

        let shell_options = versi_shell::FnmShellOptions {
            use_on_cd: self.settings.shell_options.use_on_cd,
            resolve_engines: self.settings.shell_options.resolve_engines,
            corepack_enabled: self.settings.shell_options.corepack_enabled,
        };

        let shell_type_for_callback = shell_type.clone();
        Task::perform(
            async move {
                use versi_shell::{ShellConfig, get_or_create_config_path};

                let config_path = get_or_create_config_path(&shell_type)
                    .ok_or_else(|| "No config file path found".to_string())?;

                let mut config = ShellConfig::load(shell_type.clone(), config_path)
                    .map_err(|e| e.to_string())?;

                let edit = config.add_fnm_init(&shell_options);
                if edit.has_changes() {
                    config.apply_edit(&edit).map_err(|e| e.to_string())?;
                }

                Ok::<_, String>(())
            },
            move |result| Message::ShellConfigured(shell_type_for_callback.clone(), result),
        )
    }

    pub(super) fn handle_shell_configured(
        &mut self,
        shell_type: versi_shell::ShellType,
        result: Result<(), String>,
    ) {
        if let AppState::Main(state) = &mut self.state
            && let Some(shell) = state
                .settings_state
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

    pub(super) fn update_shell_flags(&self) -> Task<Message> {
        let shell_options = versi_shell::FnmShellOptions {
            use_on_cd: self.settings.shell_options.use_on_cd,
            resolve_engines: self.settings.shell_options.resolve_engines,
            corepack_enabled: self.settings.shell_options.corepack_enabled,
        };

        Task::perform(
            async move {
                use versi_shell::ShellConfig;

                let shells = detect_shells();
                let mut updated_count = 0;

                for shell in shells {
                    if let Some(config_path) = shell.config_file
                        && let Ok(mut config) =
                            ShellConfig::load(shell.shell_type.clone(), config_path)
                        && config.has_fnm_init()
                    {
                        let edit = config.update_fnm_flags(&shell_options);
                        if edit.has_changes() {
                            config.apply_edit(&edit).map_err(|e| e.to_string())?;
                            updated_count += 1;
                        }
                    }
                }

                Ok::<_, String>(updated_count)
            },
            Message::ShellFlagsUpdated,
        )
    }
}
