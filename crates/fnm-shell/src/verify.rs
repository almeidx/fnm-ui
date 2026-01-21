use crate::config::ShellConfig;
use crate::detect::ShellType;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub enum VerificationResult {
    Configured,
    NotConfigured,
    ConfigFileNotFound,
    FunctionalButNotInConfig,
    Error(String),
}

pub async fn verify_shell_config(shell_type: &ShellType) -> VerificationResult {
    let config_files = shell_type.config_files();
    let existing_config = config_files.iter().find(|p| p.exists());

    match existing_config {
        Some(config_path) => match ShellConfig::load(shell_type.clone(), config_path.clone()) {
            Ok(config) => {
                if config.has_fnm_init() {
                    VerificationResult::Configured
                } else {
                    if functional_test(shell_type).await {
                        VerificationResult::FunctionalButNotInConfig
                    } else {
                        VerificationResult::NotConfigured
                    }
                }
            }
            Err(e) => VerificationResult::Error(e.to_string()),
        },
        None => {
            if functional_test(shell_type).await {
                VerificationResult::FunctionalButNotInConfig
            } else {
                VerificationResult::ConfigFileNotFound
            }
        }
    }
}

async fn functional_test(shell_type: &ShellType) -> bool {
    let test_command = match shell_type {
        ShellType::Bash => {
            let result = Command::new("bash")
                .args(["-i", "-c", "fnm --version"])
                .output()
                .await;
            return result.map(|o| o.status.success()).unwrap_or(false);
        }
        ShellType::Zsh => {
            let result = Command::new("zsh")
                .args(["-i", "-c", "fnm --version"])
                .output()
                .await;
            return result.map(|o| o.status.success()).unwrap_or(false);
        }
        ShellType::Fish => {
            let result = Command::new("fish")
                .args(["-c", "fnm --version"])
                .output()
                .await;
            return result.map(|o| o.status.success()).unwrap_or(false);
        }
        ShellType::PowerShell => {
            let shell = if which::which("pwsh").is_ok() {
                "pwsh"
            } else {
                "powershell"
            };
            let result = Command::new(shell)
                .args(["-Command", "fnm --version"])
                .output()
                .await;
            return result.map(|o| o.status.success()).unwrap_or(false);
        }
        ShellType::Cmd => return false,
    };
}

pub fn get_config_path_for_shell(shell_type: &ShellType) -> Option<PathBuf> {
    shell_type.config_files().into_iter().find(|p| p.exists())
}

pub fn get_or_create_config_path(shell_type: &ShellType) -> Option<PathBuf> {
    if let Some(existing) = get_config_path_for_shell(shell_type) {
        return Some(existing);
    }

    shell_type.config_files().into_iter().next()
}
