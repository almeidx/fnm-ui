use log::{debug, info, trace};
use std::path::{Path, PathBuf};

use iced::Task;

#[cfg(windows)]
use versi_core::HideWindow;
use versi_core::{FnmBackend, VersionManager, detect_fnm};
use versi_platform::EnvironmentId;
use versi_shell::detect_shells;

use crate::message::{EnvironmentInfo, InitResult, Message};
use crate::state::{AppState, EnvironmentState, MainState, OnboardingState, ShellConfigStatus};

use super::Versi;

impl Versi {
    pub(super) fn handle_initialized(&mut self, result: InitResult) -> Task<Message> {
        info!(
            "Handling initialization result: fnm_found={}, environments={}",
            result.fnm_found,
            result.environments.len()
        );

        if !result.fnm_found {
            info!("fnm not found, entering onboarding flow");
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

        let fnm_path = result.fnm_path.unwrap_or_else(|| PathBuf::from("fnm"));
        let fnm_dir = result.fnm_dir;

        self.backend_path = fnm_path.clone();
        self.backend_dir = fnm_dir.clone();

        let backend = FnmBackend::new(
            fnm_path.clone(),
            result.fnm_version.clone(),
            fnm_dir.clone(),
        );
        let backend = if let Some(dir) = fnm_dir.clone() {
            backend.with_fnm_dir(dir)
        } else {
            backend
        };
        let backend: Box<dyn VersionManager> = Box::new(backend.clone());

        let environments: Vec<EnvironmentState> = result
            .environments
            .iter()
            .map(|env_info| {
                if env_info.available {
                    EnvironmentState::new(env_info.id.clone(), env_info.fnm_version.clone())
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

        let mut main_state = MainState::new_with_environments(backend, environments);

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
            let backend = create_backend_for_environment(&env_id, &fnm_path, &fnm_dir);

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
        let check_fnm_update = self.handle_check_for_fnm_update();

        load_tasks.extend([
            fetch_remote,
            fetch_schedule,
            check_app_update,
            check_fnm_update,
        ]);

        Task::batch(load_tasks)
    }
}

pub(super) async fn initialize() -> InitResult {
    info!("Initializing application...");

    debug!("Detecting fnm installation...");
    let detection = detect_fnm().await;
    info!(
        "fnm detection result: found={}, path={:?}, version={:?}",
        detection.found, detection.path, detection.version
    );

    #[allow(unused_mut)]
    let mut environments = vec![EnvironmentInfo {
        id: EnvironmentId::Native,
        fnm_version: detection.version.clone(),
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
                    fnm_version: None,
                    available: false,
                    unavailable_reason: Some("Not running".to_string()),
                });
            } else if let Some(fnm_path) = distro.fnm_path {
                info!(
                    "Adding WSL environment: {} (fnm at {})",
                    distro.name, fnm_path
                );
                let fnm_version = get_wsl_fnm_version(&distro.name, &fnm_path).await;
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path,
                    },
                    fnm_version,
                    available: true,
                    unavailable_reason: None,
                });
            } else {
                info!(
                    "Adding unavailable WSL environment: {} (fnm not found)",
                    distro.name
                );
                environments.push(EnvironmentInfo {
                    id: EnvironmentId::Wsl {
                        distro: distro.name,
                        fnm_path: String::new(),
                    },
                    fnm_version: None,
                    available: false,
                    unavailable_reason: Some("fnm not installed".to_string()),
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
        fnm_found: detection.found,
        fnm_path: detection.path,
        fnm_dir: detection.fnm_dir,
        fnm_version: detection.version,
        environments,
    }
}

#[cfg(windows)]
async fn get_wsl_fnm_version(distro: &str, fnm_path: &str) -> Option<String> {
    use tokio::process::Command;

    let output = Command::new("wsl.exe")
        .args(["-d", distro, "--", fnm_path, "--version"])
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
        debug!("WSL {} fnm version: {}", distro, version);
        Some(version)
    } else {
        None
    }
}

pub(super) fn create_backend_for_environment(
    env_id: &EnvironmentId,
    detected_fnm_path: &Path,
    detected_fnm_dir: &Option<PathBuf>,
) -> Box<dyn VersionManager> {
    match env_id {
        EnvironmentId::Native => {
            let backend = FnmBackend::new(
                detected_fnm_path.to_path_buf(),
                None,
                detected_fnm_dir.clone(),
            );
            let backend = if let Some(dir) = detected_fnm_dir {
                backend.with_fnm_dir(dir.clone())
            } else {
                backend
            };
            Box::new(backend)
        }
        EnvironmentId::Wsl { distro, fnm_path } => {
            Box::new(FnmBackend::with_wsl(distro.clone(), fnm_path.clone()))
        }
    }
}
