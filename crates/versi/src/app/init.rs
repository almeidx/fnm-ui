use log::{debug, info, trace};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;

use versi_backend::{BackendProvider, VersionManager};
use versi_platform::EnvironmentId;
use versi_shell::detect_shells;

use crate::message::{EnvironmentInfo, InitResult, Message};
use crate::state::{AppState, EnvironmentState, MainState, OnboardingState, ShellConfigStatus};

use super::Versi;

impl Versi {
    pub(super) fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        info!(
            "Handling initialization result: backend_found={}, environments={}",
            result.backend_found,
            result.environments.len()
        );

        if !result.backend_found {
            info!("Backend not found, entering onboarding flow");
            let shells = detect_shells();
            debug!("Detected {} shells for configuration", shells.len());

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

        let backend_path = result
            .backend_path
            .unwrap_or_else(|| PathBuf::from(self.provider.name()));
        let backend_dir = result.backend_dir;

        self.backend_path = backend_path.clone();
        self.backend_dir = backend_dir.clone();

        let detection = versi_backend::BackendDetection {
            found: true,
            path: Some(backend_path.clone()),
            version: result.backend_version.clone(),
            in_path: true,
            data_dir: backend_dir.clone(),
        };
        let backend = self.provider.create_manager(&detection);

        let environments: Vec<EnvironmentState> = result
            .environments
            .iter()
            .map(|env_info| {
                if env_info.available {
                    EnvironmentState::new(env_info.id.clone(), env_info.backend_version.clone())
                } else {
                    EnvironmentState::unavailable(
                        env_info.id.clone(),
                        env_info
                            .unavailable_reason
                            .as_deref()
                            .unwrap_or("Unavailable"),
                    )
                }
            })
            .collect();

        let mut main_state =
            MainState::new_with_environments(backend, environments, self.provider.name());

        if let Some(disk_cache) = crate::cache::DiskCache::load() {
            debug!(
                "Loaded disk cache from {:?} ({} versions, schedule={})",
                disk_cache.cached_at,
                disk_cache.remote_versions.len(),
                disk_cache.release_schedule.is_some()
            );
            if !disk_cache.remote_versions.is_empty() {
                main_state.available_versions.versions = disk_cache.remote_versions;
                main_state.available_versions.loaded_from_disk = true;
            }
            if let Some(schedule) = disk_cache.release_schedule {
                main_state.available_versions.schedule = Some(schedule);
            }
        }

        self.state = AppState::Main(main_state);

        let mut load_tasks: Vec<Task<Message>> = Vec::new();

        for env_info in &result.environments {
            if !env_info.available {
                debug!(
                    "Skipping load for unavailable environment: {:?}",
                    env_info.id
                );
                continue;
            }

            let env_id = env_info.id.clone();
            let backend = create_backend_for_environment(
                &env_id,
                &backend_path,
                &backend_dir,
                &self.provider,
            );

            load_tasks.push(Task::perform(
                async move {
                    let versions = backend.list_installed().await.unwrap_or_default();
                    (env_id, versions)
                },
                move |(env_id, versions)| Message::EnvironmentLoaded { env_id, versions },
            ));
        }

        let fetch_remote = self.handle_fetch_remote_versions();
        let fetch_schedule = self.handle_fetch_release_schedule();
        let check_app_update = self.handle_check_for_app_update();
        let check_backend_update = self.handle_check_for_backend_update();

        load_tasks.extend([
            fetch_remote,
            fetch_schedule,
            check_app_update,
            check_backend_update,
        ]);

        Task::batch(load_tasks)
    }
}

pub(super) async fn initialize(provider: Arc<dyn BackendProvider>) -> InitResult {
    info!("Initializing application...");

    debug!("Detecting backend installation...");
    let detection = provider.detect().await;
    info!(
        "Backend detection result: found={}, path={:?}, version={:?}",
        detection.found, detection.path, detection.version
    );

    #[allow(unused_mut)]
    let mut environments = vec![EnvironmentInfo {
        id: EnvironmentId::Native,
        backend_version: detection.version.clone(),
        available: true,
        unavailable_reason: None,
    }];

    #[cfg(windows)]
    {
        use versi_platform::detect_wsl_distros;
        info!("Running on Windows, detecting WSL distros...");
        let distros = detect_wsl_distros();
        debug!(
            "WSL distros found: {:?}",
            distros.iter().map(|d| &d.name).collect::<Vec<_>>()
        );

        for distro in distros {
            if !distro.is_running {
                info!(
                    "Adding unavailable WSL environment: {} (not running)",
                    distro.name
                );
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: String::new(),
                    },
                    backend_version: None,
                    available: false,
                    unavailable_reason: Some("Not running".to_string()),
                });
            } else if let Some(backend_path) = distro.fnm_path {
                info!(
                    "Adding WSL environment: {} (backend at {})",
                    distro.name, backend_path
                );
                let backend_version = get_wsl_backend_version(&distro.name, &backend_path).await;
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: backend_path,
                    },
                    backend_version,
                    available: true,
                    unavailable_reason: None,
                });
            } else {
                let backend_name = provider.name();
                info!(
                    "Adding unavailable WSL environment: {} ({} not found)",
                    distro.name, backend_name
                );
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: String::new(),
                    },
                    backend_version: None,
                    available: false,
                    unavailable_reason: Some(format!("{} not installed", backend_name)),
                });
            }
        }
    }

    info!(
        "Initialization complete with {} environments",
        environments.len()
    );
    for (i, env) in environments.iter().enumerate() {
        trace!("  Environment {}: {:?}", i, env);
    }

    InitResult {
        backend_found: detection.found,
        backend_path: detection.path,
        backend_dir: detection.data_dir,
        backend_version: detection.version,
        environments,
    }
}

#[cfg(windows)]
async fn get_wsl_backend_version(distro: &str, backend_path: &str) -> Option<String> {
    use tokio::process::Command;
    use versi_core::HideWindow;

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", backend_path, "--version"])
        .hide_window()
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .trim()
            .strip_prefix("fnm ")
            .unwrap_or(version_str.trim())
            .to_string();
        debug!("WSL {} backend version: {}", distro, version);
        Some(version)
    } else {
        None
    }
}

pub(super) fn create_backend_for_environment(
    env_id: &EnvironmentId,
    detected_path: &Path,
    detected_dir: &Option<PathBuf>,
    provider: &Arc<dyn BackendProvider>,
) -> Box<dyn VersionManager> {
    match env_id {
        EnvironmentId::Native => {
            let detection = versi_backend::BackendDetection {
                found: true,
                path: Some(detected_path.to_path_buf()),
                version: None,
                in_path: true,
                data_dir: detected_dir.clone(),
            };
            provider.create_manager(&detection)
        }
        EnvironmentId::Wsl { distro, fnm_path } => {
            provider.create_manager_for_wsl(distro.clone(), fnm_path.clone())
        }
    }
}
